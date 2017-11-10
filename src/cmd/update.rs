use {Error, Result};
use package::{self, Package};
use num_cpus;
use docopt::Docopt;
use git;
use task::TaskManager;

const USAGE: &'static str = "
Update plugins.

Usage:
    pack update
    pack update [options]
    pack update [options] <plugin>...
    pack update -h | --help

Options:
    -p, --packfile          Regenerates the '_pack' file to combine all plugins
                            configrations.
    -s, --skip SKIP         Comma separated list of plugins to skip
    -j, --threads THREADS   Update plugins concurrently
    -h, --help              Display this message
";

#[derive(Debug, RustcDecodable)]
struct UpdateArgs {
    arg_plugin: Vec<String>,
    flag_threads: Option<usize>,
    flag_packfile: Option<bool>,
    flag_skip: String,
}

pub fn execute(args: &[String]) {
    let mut argv = vec!["pack".to_string(), "update".to_string()];
    argv.extend_from_slice(args);

    let args: UpdateArgs = Docopt::new(USAGE)
        .and_then(|d| d.argv(argv)
        .decode())
        .unwrap_or_else(|e| e.exit());

    if args.flag_packfile.is_some() {
        if let Err(e) = update_packfile() {
            die!("Err: {}", e);
        }
        return
    }

    let threads = args.flag_threads.unwrap_or(num_cpus::get());
    if threads < 1 {
        die!("Threads should be greater than 0");
    }
    let skip = args.flag_skip.split(",")
        .map(|x| String::from(x.trim()))
        .filter(|x| !x.is_empty())
        .collect();

    if let Err(e) = update_plugins(args.arg_plugin, threads, skip) {
        die!("Err: {}", e);
    }
}

fn update_packfile() -> Result<()> {
    println!("Update _pack file for all plugins.");
    let mut packs = package::fetch()?;

    packs.sort_by(|a, b| a.name.cmp(&b.name));
    package::update_pack_plugin(&packs)?;

    Ok(())
}

fn update_plugins(plugins: Vec<String>, threads: usize, skip: Vec<String>) -> Result<()> {
    let mut packs = package::fetch()?;

    let mut manager = TaskManager::new(threads);
    if plugins.is_empty() {
        for pack in packs.iter() {
            if skip.iter().any(|x| pack.name.contains(x)) {
                println!("Skip {}", pack.name);
                continue
            }
            manager.add(pack.clone());
        }
    } else {
        for pack in packs.iter().filter(|x| plugins.contains(&x.name)) {
            manager.add(pack.clone());
        }
    }
    manager.run(update_plugin);

    packs.sort_by(|a, b| a.name.cmp(&b.name));

    package::update_pack_plugin(&packs)?;

    Ok(())
}

fn update_plugin(pack: &Package) -> Result<()> {
    let path = pack.path();
    if !path.is_dir() {
        Err(Error::PluginNotInstalled)
    } else if pack.local {
        Err(Error::SkipLocal)
    } else {
        git::update(&pack.name, &path)
    }
}
