mod options;

fn main() {
    let options = match options::read() {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    if options.help {
        println!(
            "plugin-eval-legibility [--root PROJECT] [--registry docs/legibility/registry.json] [--inventory]"
        );
        return;
    }
    let output = plugin_eval_legibility::run(&options.root, &options.registry, options.inventory);
    println!("{}", serde_json::to_string(&output).expect("audit JSON"));
    std::process::exit(if output["passed"] == true { 0 } else { 1 });
}
