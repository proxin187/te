mod editor;

use argin::Argin;

use std::process;


fn cli() -> Argin {
    let mut arg = Argin::new();
    arg.add_positional_arg();
    arg.add_positional_arg();
    arg.parse()
}

fn main() {
    let args = cli();
    if args.pos_arg.len() < 2 {
        println!("Usage: te [FILE]");
        process::exit(1);
    }

    // create new editor instance
    let mut edit = match editor::Editor::new(&args.pos_arg[1]) {
        Ok(edit) => edit,
        Err(err) => {
            println!("Failed to create new editor instance -> `{}`", err.to_string());
            process::exit(1);
        },
    };

    if let Err(_) = edit.open_file(&args.pos_arg[1]) {
        edit.log(&format!("Failed to open `{}`", args.pos_arg[1]));
    }

    if let Err(err) = edit.run() {
        println!("Failed to run main loop: `{}`", err.to_string());
    }
}


