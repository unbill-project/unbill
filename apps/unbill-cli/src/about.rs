use std::io::Write;

// sirno:witness:about-page:begin
pub fn write(output: &mut impl Write, json: bool) -> anyhow::Result<()> {
    let about = unbill_about::content();
    if json {
        writeln!(output, "{}", unbill_about::CONTENT_JSON)?;
        return Ok(());
    }
    writeln!(output, "{}\n{}\n", about.name, about.tagline)?;
    writeln!(output, "{}\n", about.summary)?;
    writeln!(output, "{}\n", about.principles.join(" · "))?;
    for section in &about.sections {
        writeln!(output, "{}\n", section.title)?;
        for paragraph in &section.paragraphs {
            writeln!(output, "{paragraph}\n")?;
        }
        if let Some(link) = &section.link {
            writeln!(output, "{}: {}\n", link.label, link.url)?;
        }
    }
    Ok(())
}
// sirno:witness:about-page:end
