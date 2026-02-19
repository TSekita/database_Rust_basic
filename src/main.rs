use database_rust_basic::Database;
use std::env;

fn print_usage() {
    eprintln!(
        "Usage:\n  db <file> set <key> <value>\n  db <file> get <key>\n  db <file> delete <key>\n  db <file> list\n  db <file> compact"
    );
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_usage();
        return Ok(());
    }

    let file = &args[1];
    let command = &args[2];
    let mut db = Database::open(file)?;

    match command.as_str() {
        "set" if args.len() == 5 => {
            db.set(args[3].clone(), args[4].clone())?;
            println!("OK");
        }
        "get" if args.len() == 4 => match db.get(&args[3]) {
            Some(value) => println!("{value}"),
            None => println!("(nil)"),
        },
        "delete" if args.len() == 4 => {
            let deleted = db.delete(&args[3])?;
            if deleted {
                println!("OK");
            } else {
                println!("(not found)");
            }
        }
        "list" if args.len() == 3 => {
            for (key, value) in db.list() {
                println!("{key}={value}");
            }
        }
        "compact" if args.len() == 3 => {
            db.compact()?;
            println!("OK");
        }
        _ => print_usage(),
    }

    Ok(())
}
