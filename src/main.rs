use std::env::args;
use parser::parse;

fn help() {
    println!("
Usage
-----

echo <file path>

For more information see https://github.com/togglebyte/echo
");
}

fn main() -> anyhow::Result<()> {
    let mut args = args().skip(1);
    let Some(path) = args.next() else {
        help();
        return Ok(());
    };

    let comment = args.next().unwrap_or("//".into());

    let code = std::fs::read_to_string(path)?;
    let instructions = parse(&code, &comment)?;
    let instructions = vm::compile(instructions);
    ui::run(instructions);
    Ok(())
}
