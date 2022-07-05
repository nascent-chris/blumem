use anyhow::Result;
use boolinator::Boolinator;
use std::{
    fs::{self, File, OpenOptions},
    io::{prelude::*, BufReader, ErrorKind, Seek, SeekFrom},
    ops::Not,
    path::{self, Component},
};
use thiserror::Error;

use crate::search_results::SearchResult;
//                          address 1,2                    perms 3,4,5,6            offset           dev                           inode     pathname 7
const MAPS_REGEX: &str = r"([0-9A-Fa-f]+)-([0-9A-Fa-f]+) ([-r])([-w])([-x])([-ps]) (?:[0-9A-Fa-f]+) (?:[0-9A-Fa-f]+:[0-9A-Fa-f]+) (?:\d+)\s+(.*)?";

#[derive(Debug)]
#[allow(unused)]
pub struct MemRegion {
    start_addr: u64,
    end_addr: u64,
    size: usize,
    readable: bool,
    writeable: bool,
    execable: bool,
    private: bool,
    shared: bool,
    name: Option<String>,
}

#[derive(Error, Debug)]
pub enum MemRegionErr {
    #[error("empty memory regions")]
    Empty,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Proc {
    name: String,
    pid: u32,
    path: path::PathBuf,
    regions: Option<Vec<MemRegion>>,
    mem: Option<File>,
}

impl Proc {
    pub fn search_new(&mut self, val: &[u8]) -> Result<Vec<SearchResult>> {
        // Explicitly use the bytes regex
        use regex::bytes::RegexBuilder;

        let valstr = val
            .iter()
            .map(|b| format!("\\x{:02x}", b))
            .collect::<String>();

        let re = RegexBuilder::new(&valstr)
            .unicode(false)
            .dot_matches_new_line(true)
            .case_insensitive(false)
            .build()?;

        let regions = self.regions.as_ref().ok_or(MemRegionErr::Empty)?;

        let results = regions
            .iter()
            .filter_map(|region| {
                self.mem.as_mut().map(|memfile| {
                    memfile
                        .seek(SeekFrom::Start(region.start_addr))
                        .and_then(|_| {
                            let mut reader = BufReader::with_capacity(region.size, memfile);
                            let filled = reader.fill_buf()?;
                            let found = re
                                .find_iter(filled)
                                .map(|m| region.start_addr + m.start() as u64)
                                .collect::<Vec<_>>();

                            found.is_empty().not().as_result(
                                SearchResult {
                                    module: region.name.clone(),
                                    results: found,
                                },
                                std::io::Error::new(ErrorKind::Other, "oh no!"),
                            )
                        })
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        Ok(results)
    }

    pub fn new(name: String, pid: u32) -> Self {
        Self {
            name,
            pid,
            path: path::Path::new("/proc").join(&pid.to_string()),
            regions: None,
            mem: None,
        }
    }

    pub fn open_mem(&mut self) -> Result<()> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(self.path.join("mem"))?;
        self.mem = Some(f);
        Ok(())
    }

    // pub fn parse_maps(&self) -> Result<Vec<MemRegion>> {
    pub fn parse_maps(&mut self) -> Result<()> {
        use regex::RegexBuilder;
        let buf = File::open(self.path.join("maps")).map(BufReader::new)?;

        let re = RegexBuilder::new(MAPS_REGEX)
            .unicode(true)
            .dot_matches_new_line(true)
            .case_insensitive(false)
            .build()?;

        let res = buf
            .lines()
            .into_iter()
            .flatten()
            .filter_map(|line| {
                re.captures(&line).map(|cap| {
                    let start_addr = cap
                        .get(1)
                        .and_then(|v| u64::from_str_radix(v.as_str(), 16).ok())?;
                    let end_addr = cap
                        .get(2)
                        .and_then(|v| u64::from_str_radix(v.as_str(), 16).ok())?;
                    let [readable, writeable, execable, private, shared] =
                        [(3, "r"), (4, "w"), (5, "x"), (6, "p"), (6, "s")]
                            .map(|(i, s)| cap.get(i).map(|v| v.as_str() == s).unwrap_or(false));
                    let name = cap.get(7).and_then(|v| {
                        v.as_str()
                            .is_empty()
                            .not()
                            .as_option()
                            .map(|_| v.as_str().to_string())
                    });
                    Some(MemRegion {
                        start_addr,
                        end_addr,
                        size: (end_addr - start_addr) as usize,
                        readable,
                        writeable,
                        execable,
                        private,
                        shared,
                        name,
                    })
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        self.regions = Some(res.is_empty().not().as_result(res, MemRegionErr::Empty)?);

        Ok(())
    }
}

pub fn find_proc(proc_name: &str) -> Result<Vec<Proc>> {
    fs::read_dir("/proc/")
        .map(|dir| {
            dir.flatten()
                .map(|dir| dir.path())
                .flat_map(|path| std::fs::read_to_string(path.join("cmdline")).map(|s| (path, s)))
                .filter(|(_, s)| !s.is_empty() && s.contains(proc_name))
                .flat_map(|(p, s)| {
                    p.components()
                        .last()
                        .and_then(|c| match c {
                            Component::Normal(c) => c.to_string_lossy().parse::<u32>().ok(),
                            _ => None,
                        })
                        .map(|pid| Proc::new(s.trim().replace('\x00', ""), pid))
                })
                .collect()
        })
        .map_err(Into::into)
}
