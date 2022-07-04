mod proc;

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

    let search_str = 1234.567f32.to_le_bytes();

    let search_res = procs
        .iter_mut()
        .map(|proc| proc.search(&search_str))
        .collect::<Vec<_>>();

    println!("search_res: {:#X?}", search_res);

    Ok(())
}
