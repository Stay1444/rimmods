use anyhow::{Result, Ok, Error, bail};
use fs_extra::dir::CopyOptions;

use std::{
    path::{PathBuf, Path}, 
    fs::{OpenOptions, remove_dir_all, create_dir, self}, 
    io::{BufReader, BufRead, Write}, 
    process::{Command, ChildStdout, Stdio, ChildStdin}, 
    time::Duration};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name to the rimworld mods folder
    /// Example: .wine/drive_c/Games/RimWorld/Mods/
    #[arg(short, long)]
    pub mods_dir: PathBuf,

    /// The steam directory where the rimworld mods will be downloaded.
    /// Example: .local/share/Steam/steamapps/workshop/content/294100/
    #[arg(short, long)]
    pub steam_dir: PathBuf,

    /// Redownload of all mods, even if they already exist
    #[arg(short, long)]
    pub clean: bool
}

pub const RIMWORLD_GAME_ID: u64 = 294100;

struct RimMod {
    pub name: String,
    pub id: i64,
    pub _url: String
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !args.mods_dir.is_dir() {
        return Err(Error::msg("mods_dir expected to be a directory and exist!"));
    }

    if !args.steam_dir.is_dir() {
        return Err(Error::msg("steam_dir expected to be a directory and exist!"));
    }

    let modlist_path = args.mods_dir.join("mods.txt");

    if !modlist_path.is_file() {
        return Err(Error::msg(format!("Error! mods.txt file not found in {:?}", args.mods_dir)));

    }

    println!("Spawning SteamCMD...");

    let steamcmd = Command::new("steamcmd")
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()?;

    let mut steam_stdin = steamcmd.stdin.unwrap();
    let mut steam_stdout = BufReader::new(steamcmd.stdout.unwrap());

    println!("Waiting for SteamCMD login...");
    println!("-----------------------------");

    writeln!(steam_stdin, "login anonymous")?;
    loop {
        let mut line = String::new();
        steam_stdout.read_line(&mut line)?;
        let line = line.replace('\n', "");

        println!("Steam -> {}", line);
        if line == "Waiting for user info...OK" {
            println!("-----------------------------");
            println!("Logged into SteamCMD correctly");
            break;
        }   
    }

    println!("Loading mods from {modlist_path:?}..");

    let mods = load_mods(&modlist_path)?;

    println!("Found {} mods", {mods.len()});

    for rimmod in mods {
        let mod_path = args.mods_dir.join(format!("{}", rimmod.id));
        let steam_path = args.steam_dir.join(format!("{}", rimmod.id));

        if mod_path.is_dir() {
            if args.clean {
                println!("Removing {:?} from rimworld folder (clean)", mod_path);
                remove_dir_all(&mod_path)?;
            } else if fs::read_dir(&mod_path)?.count() > 0 {
                println!("Mod {} ({}) already exists. Skipping...", rimmod.name, rimmod.id);
                continue;
            }
        }

        if steam_path.is_dir() {
            if args.clean {
                println!("Removing {:?} from steam folder (clean)", steam_path);
                remove_dir_all(&steam_path)?;
            } else if fs::read_dir(&steam_path)?.count() > 0 {
                println!("Mod {} ({}) already downloaded. Moving...", rimmod.name, rimmod.id);
                if !mod_path.is_dir() {
                    create_dir(&mod_path)?;
                }
                fs_extra::copy_items(&[steam_path], mod_path, &CopyOptions::new())?;
                continue;
            }
        }

        println!("Downloading {} ({})...", rimmod.name, rimmod.id);

        let mut success = false;
        for i in 0..3 {
            match steamcmd_download(&mut steam_stdin, &mut steam_stdout, &rimmod) {
                Err(_) => {
                    println!("Mod download failed, retrying ({})", i);
                    continue;
                },
                _ => {
                    success = true;
                    break;
                },
            };
        }

        if !success {
            bail!("Error downloading mod {}", rimmod.name);
        }

        for i in 0..10 {
            std::thread::sleep(Duration::from_millis(i * 250));

            if steam_path.is_dir() {
                break;
            }
        }

        println!("Downloaded {} ({})", rimmod.name, rimmod.id);

        if !mod_path.is_dir() {
            create_dir(&mod_path)?;
        }

        fs_extra::copy_items(&[steam_path], mod_path, &CopyOptions::new())?;
    }

    println!("All mods checked out. Bye!");
    
    Ok(())
}

fn steamcmd_download(steam_stdin: &mut ChildStdin, steam_stdout: &mut BufReader<ChildStdout>, rimmod: &RimMod) -> Result<()> {
    println!("-----------------------------");

    writeln!(steam_stdin, "workshop_download_item {} {}", RIMWORLD_GAME_ID, rimmod.id)?;
    loop {
        let mut line = String::new();
        steam_stdout.read_line(&mut line)?;
        let line = line.replace('\n', "");

        println!("Steam -> {}", line);
        if line.starts_with(&format!("Success. Downloaded item {} to", rimmod.id)) {
            println!("-----------------------------");
            break;
        }

        if line.starts_with(&format!("ERROR! Download item {} failed", rimmod.id)) {
            println!("-----------------------------");
            bail!("Error downloading mod {} ({})", rimmod.name, rimmod.id);
        }
    }

    Ok(())
}

fn load_mods(list_path: &Path) -> Result<Vec<RimMod>> {
    let file = OpenOptions::new()
        .read(true)
        .open(list_path)?;

    let reader = BufReader::new(file);
    
    let mut mods = Vec::new();

    for row in reader.lines() {
        let parts: Vec<String> = row?.split(' ')
            .map(|x| x.to_owned())
            .collect();

        let url = parts[0].to_owned();
        let name = parts[1..].join(" ").to_owned();


        let url_split = url.split("?id=")
            .collect::<Vec<&str>>();

        let id: i64 = {
            let id_str = *url_split.get(1).expect("Mod Id");
            id_str.parse::<i64>()?
        };

        println!("Found mod {name} - ({url})");

        mods.push(RimMod { 
            name, 
            id, 
            _url: url
        });
    }

    Ok(mods)
}