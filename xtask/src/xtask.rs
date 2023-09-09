use anyhow::{anyhow, Context, Result};

use fn_error_context::context;
use std::fmt::Write as WriteFmt;
use xshell::{cmd, Shell};

const INSTALL_BUTANE: &str = include_str!("bootc-install.yml");

fn main() {
    if let Err(e) = try_main() {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
}

#[allow(clippy::type_complexity)]
const TASKS: &[(&str, fn(&Shell, args: &[String]) -> Result<()>)] =
    &[("inst-coreos", prepare_coreos_install_iso)];

fn try_main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let task = args.next();
    let args = args.collect::<Vec<_>>();
    let sh = xshell::Shell::new().context("Creating shell")?;
    if let Some(cmd) = task.as_deref() {
        let f = TASKS
            .iter()
            .find_map(|(k, f)| (*k == cmd).then_some(*f))
            .unwrap_or(print_help);
        f(&sh, args.as_slice())?;
    } else {
        print_help(&sh, &[])?;
    }
    Ok(())
}

#[context("Generating ISO")]
fn prepare_coreos_install_iso(sh: &Shell, args: &[String]) -> Result<()> {
    let mut args = args.iter();
    let srciso = args.next().ok_or_else(|| anyhow!("Expected SRCISO"))?;
    let source = args.next().ok_or_else(|| anyhow!("Expected SOURCE"))?;
    let destiso = args.next().ok_or_else(|| anyhow!("Expected DESTISO"))?;
    // There will only be one virtio-block device here
    let target_device = "/dev/vda";
    cmd!(sh, "cp -f --reflink=auto {srciso} {destiso}").run()?;
    let mut buf = INSTALL_BUTANE.to_string();
    write!(
        buf,
        r##"
storage:
  files:
    - path: /etc/bootc-install.env
      contents:
        inline: |
          BOOTC_SOURCE_IMAGE={source}
          BOOTC_INSTALL_ARGS={target_device}
"##
    )
    .unwrap();
    let ign = cmd!(sh, "butane").stdin(&buf).read()?;
    cmd!(sh, "coreos-installer iso ignition embed {destiso}")
        .stdin(&ign)
        .run()?;
    Ok(())
}

fn print_help(_sh: &Shell, _args: &[String]) -> Result<()> {
    println!("Tasks:");
    for (name, _) in TASKS {
        println!("  - {name}");
    }
    Ok(())
}
