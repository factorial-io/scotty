use owo_colors::OwoColorize;

/// RGB color representation
struct RgbColor {
    r: u8,
    g: u8,
    b: u8,
}

// Logo color - #005FA3 (blue)
const LOGO_COLOR: RgbColor = RgbColor {
    r: 0x00,
    g: 0x5F,
    b: 0xA3,
};

// Text color for version and copyright - #D1481A (orange-red)
const TEXT_COLOR: RgbColor = RgbColor {
    r: 0xD1,
    g: 0x48,
    b: 0x1A,
};

/// Print the scotty logo with version and copyright information
///
/// The logo is displayed in color #005FA3 (blue) and the version/copyright
/// text in color #D1481A (orange-red), aligned to the right.
pub fn print_logo() {
    let version = env!("CARGO_PKG_VERSION");

    // Lines of the ASCII art logo
    let line1 = "                   ██    ██         ";
    let line2 = "  ████ ████ ████ ████  ████   ██  ██";
    let line3 = "████   ██   ████   ████  ████ ██████";
    let line4 = "                                ████";

    // Info text
    let version_text = format!("   scotty v{}", version);
    let copyright_text = "   Made with ❤️ by factorial.io";

    // Print newline before logo
    println!();

    // Print the logo with colored text
    println!(
        "{}",
        line1.truecolor(LOGO_COLOR.r, LOGO_COLOR.g, LOGO_COLOR.b)
    );
    println!(
        "{}{}",
        line2.truecolor(LOGO_COLOR.r, LOGO_COLOR.g, LOGO_COLOR.b),
        version_text.truecolor(TEXT_COLOR.r, TEXT_COLOR.g, TEXT_COLOR.b)
    );
    println!(
        "{}{}",
        line3.truecolor(LOGO_COLOR.r, LOGO_COLOR.g, LOGO_COLOR.b),
        copyright_text.truecolor(TEXT_COLOR.r, TEXT_COLOR.g, TEXT_COLOR.b)
    );
    println!(
        "{}",
        line4.truecolor(LOGO_COLOR.r, LOGO_COLOR.g, LOGO_COLOR.b)
    );
    println!();
}
