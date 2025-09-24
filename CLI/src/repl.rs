use std::io::{self, Write as _};

use ninja::{
    dsl::{DslContext, execute_commands},
    manager::ShurikenManager,
};

pub fn get_input(prompt: &str) -> Result<String, io::Error> {
    // print prompt exactly as given
    let mut out = io::stdout();
    out.write_all(prompt.as_bytes())?;
    out.flush()?;

    // read input
    let mut input = String::new();
    let n = io::stdin().read_line(&mut input)?;
    if n == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "stdin closed"));
    }

    // strip trailing \r\n (Windows) or \n (Unix)
    while matches!(input.chars().last(), Some('\n' | '\r')) {
        input.pop();
    }
    Ok(input)
}

pub async fn repl_mode() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ShurikenManager::new().await?;
    let rt = DslContext::new(manager);
    println!("Welcome to REPL mode of Ninja. if you want to exit, use .exit\n");
    loop {
        let mut prompt = "ninja".to_string();
        match rt.selected.read().await.clone() {
            Some(e) => prompt.push_str(format!(" / {}> ", e).as_str()),
            None => prompt.push_str(" > "),
        };

        let input = get_input(prompt.as_str())?;
        let res = execute_commands(&rt, input.clone()).await?;

        for line in res {
            println!("{}", line);
        }

        if input == ".exit" {
            return Ok(());
        }
    }
}
