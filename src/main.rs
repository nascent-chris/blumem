mod proc;
mod search_results;

use std::io;

use anyhow::Result;

use crate::proc::find_proc;

fn main() -> Result<()> {
    let mut procs = find_proc("dummy")?;

    println!("procs {:#?}", procs);

    let _maps = procs
        .iter_mut()
        .map(|proc| proc.parse_maps())
        .collect::<Vec<_>>();

    println!("maps: {:#X?}", _maps);

    let mem = procs
        .iter_mut()
        .map(|proc| proc.open_mem())
        .collect::<Vec<_>>();

    println!("mem: {:#X?}", mem);

    println!("procs {:#X?}", procs);

    loop {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
        let search_val: f32 = buffer.trim().parse()?;
        let search_str = search_val.to_le_bytes();

        println!("search_str: {:X?}", search_str);

        let search_res = procs
            .iter_mut()
            .flat_map(|proc| proc.search_new(&search_str))
            .flatten()
            .collect::<Vec<_>>();

        println!("search_res: {:#X?}", search_res);
    }

    // Ok(())
}
