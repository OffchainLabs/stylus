#[derive(PartialEq)]
pub enum StylusCheck {
    CompressedSize,
}

impl From<&str> for StylusCheck {
    fn from(value: &str) -> Self {
        match value {
            "compressed-size" => StylusCheck::CompressedSize,
            _ => panic!(
                "Invalid Stylus middleware check: {}, allowed middlewares are: foo",
                value
            ),
        }
    }
}

pub fn run_checks(disabled: Option<Vec<StylusCheck>>) -> eyre::Result<()> {
    // Compile the Rust program at the current working directory into WASM using
    // Cargo and then instrument the WASM code with Stylus. If any of the checks
    // are disabled, we avoid runnng it.
    let _check_compressed_size = disabled
        .as_ref()
        .map(|d| !d.contains(&StylusCheck::CompressedSize))
        .unwrap_or(true);
    Ok(())
}