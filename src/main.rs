mod proc;

use std::fs;

use anyhow::Result;
use proc::Proc;

fn main() -> Result<()> {
    let procs = find_proc("dummy")?;

    println!("{:#?}", procs);

    let _maps = procs
        .iter()
        .map(|proc| proc.parse_maps())
        .collect::<Vec<_>>();

    println!("maps: {:#X?}", _maps);

    Ok(())
}

fn find_proc(proc_name: &str) -> Result<Vec<Proc>> {
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
                            std::path::Component::Normal(c) => {
                                c.to_string_lossy().parse::<u32>().ok()
                            }
                            _ => None,
                        })
                        .map(|pid| Proc::new(s.trim().replace('\x00', ""), pid))
                })
                .collect()
        })
        .map_err(Into::into)
}
