use anyhow::Result;
use boolinator::Boolinator;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Not,
    path,
};
use thiserror::Error;
//                          address 1,2                    perms 3,4,5,6            offset           dev                           inode     pathname 7
const MAPS_REGEX: &str = r"([0-9A-Fa-f]+)-([0-9A-Fa-f]+) ([-r])([-w])([-x])([-ps]) (?:[0-9A-Fa-f]+) (?:[0-9A-Fa-f]+:[0-9A-Fa-f]+) (?:\d+)\s+(.*)?";

#[derive(Debug)]
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
pub(crate) struct Proc {
    pub name: String,
    pub pid: u32,
    path: path::PathBuf,
}

impl Proc {
    pub fn new(name: String, pid: u32) -> Self {
        Self {
            name,
            pid,
            path: path::Path::new("/proc").join(&pid.to_string()),
        }
    }

    pub fn parse_maps(&self) -> Result<Vec<MemRegion>> {
        use regex::RegexBuilder;
        let buf = File::open(self.path.join("maps")).map(BufReader::new)?;
        let mut builder = RegexBuilder::new(MAPS_REGEX);
        builder
            .unicode(true)
            .dot_matches_new_line(true)
            .case_insensitive(false);
        let re = builder.build()?;

        let res = buf
            .lines()
            .into_iter()
            .flatten()
            .map(|line| {
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
            .flatten()
            .collect::<Vec<_>>();

        res.is_empty()
            .not()
            .as_result(res, MemRegionErr::Empty)
            .map_err(Into::into)
    }
}
