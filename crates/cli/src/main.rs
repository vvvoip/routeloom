use routeloom_core::{Flow, Network, Policy};
use std::{env, fs::File, io::Read, process::ExitCode};

fn run(args: &[String], zh: bool) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args[0] == "--help" {
        println!(
            "{}",
            if zh {
                "RouteLoom 开发工具 — 不会代理或修改网络流量"
            } else {
                "RouteLoom development tools — no traffic is proxied or modified"
            }
        );
        println!("check <rules>\nexplain <rules> <host> [resolved-ip|-] [port] [tcp|udp]\npac <rules> <127.0.0.1:port>\n--lang en|zh-CN <command>");
        return Ok(());
    }
    let command = args[0].as_str();
    let valid = match command {
        "check" => args.len() == 2,
        "explain" => (3..=6).contains(&args.len()),
        "pac" => args.len() == 3,
        _ => false,
    };
    if !valid {
        return Err("invalid command or argument count; use --help".into());
    }
    let mut source = String::new();
    File::open(&args[1])?
        .take(1_048_577)
        .read_to_string(&mut source)?;
    let policy = Policy::parse(&source)?;
    match command {
        "check" => println!(
            "{}: {}",
            if zh {
                "规则有效，规则数"
            } else {
                "Valid policy; rule count"
            },
            policy.rule_count()
        ),
        "explain" => {
            let ip = args
                .get(3)
                .filter(|v| v.as_str() != "-")
                .map(|v| v.parse())
                .transpose()?;
            let port = args.get(4).map(|v| v.parse()).transpose()?.unwrap_or(443);
            let network = args
                .get(5)
                .map(|v| v.parse())
                .transpose()?
                .unwrap_or(Network::Tcp);
            let decision = policy.evaluate(&Flow::new(&args[2], ip, port, network)?);
            println!(
                "{}: {}\n{}: {}\n{}: {}",
                if zh { "决策" } else { "Action" },
                decision.action,
                if zh { "匹配行" } else { "Matched line" },
                decision.line,
                if zh { "默认规则" } else { "Fallback" },
                decision.is_fallback
            );
        }
        "pac" => print!("{}", policy.to_pac(args[2].parse()?)?),
        _ => unreachable!(),
    }
    Ok(())
}
fn main() -> ExitCode {
    let mut args: Vec<_> = env::args().skip(1).collect();
    let mut zh = false;
    if args.first().is_some_and(|a| a == "--lang") {
        if args.len() < 2 || !matches!(args[1].as_str(), "en" | "zh-CN") {
            eprintln!("Unsupported language / 不支持的语言: en, zh-CN");
            return ExitCode::FAILURE;
        }
        zh = args[1] == "zh-CN";
        args.drain(..2);
    }
    match run(&args, zh) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "{}: {error}",
                if zh {
                    "错误（开发诊断）"
                } else {
                    "Error"
                }
            );
            ExitCode::FAILURE
        }
    }
}
