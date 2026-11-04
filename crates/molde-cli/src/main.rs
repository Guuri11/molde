//! Binario: lsp, check, stats, candidates, scaffold.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        println!("{}", molde_cli::USAGE);
        return;
    }
    let paths = molde_cli::Paths::from_vars(|k| std::env::var(k).ok());
    let mut engine = molde_cli::engine(&paths).unwrap_or_else(|e| {
        eprintln!("molde: {e}");
        std::process::exit(1)
    });
    if args.first().map(String::as_str) == Some("lsp") {
        let rt = tokio::runtime::Runtime::new().expect("runtime de tokio");
        rt.block_on(molde_lsp::serve_stdio(engine));
        return;
    }
    let cwd = std::env::current_dir().expect("directorio actual");
    let code = molde_cli::run(
        &args,
        &cwd,
        &mut engine,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    std::process::exit(code);
}
