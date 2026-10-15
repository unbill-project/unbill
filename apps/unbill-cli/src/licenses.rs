use anyhow::{Result, bail};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
};

// sirno:witness:unbill-license:begin
pub fn write(output: &mut impl Write, software: Option<&str>, all: bool, json: bool) -> Result<()> {
    let catalog = unbill_license::dictionary();
    let selected = if let Some(name) = software {
        let Some(records) = catalog.get(name) else {
            bail!("unknown software '{name}'; run `unbill licenses` to list software");
        };
        BTreeMap::from([(name, records)])
    } else {
        catalog
            .iter()
            .map(|(name, records)| (name.as_str(), records))
            .collect()
    };
    if json {
        serde_json::to_writer_pretty(&mut *output, &selected)?;
        writeln!(output)?;
        return Ok(());
    }
    writeln!(
        output,
        "Rust workspace legal catalog (all features and platforms)"
    )?;
    writeln!(output)?;
    if software.is_none() && !all {
        for records in selected.values() {
            for record in *records {
                writeln!(
                    output,
                    "{} {} — {}",
                    record.name, record.version, record.license_expression
                )?;
            }
        }
        writeln!(output, "\nView full texts: unbill licenses <software>")?;
        writeln!(output, "View every text: unbill licenses --all")?;
        return Ok(());
    }
    for records in selected.values() {
        for record in *records {
            writeln!(
                output,
                "{} {}\nLicense: {}",
                record.name, record.version, record.license_expression
            )?;
            if let Some(repository) = &record.repository {
                writeln!(output, "Repository: {repository}")?;
            }
            if let Some(source) = &record.source_archive {
                writeln!(output, "Source archive: {source}")?;
            } else if let Some(source) = &record.source {
                writeln!(output, "Source: {source}")?;
            }
            if !record.authors.is_empty() {
                writeln!(output, "Package authors: {}", record.authors.join(", "))?;
            }
            let mut printed = BTreeSet::new();
            // Show original files first so notices and actual copyright holders
            // are visible, including documents not identified by the scanner.
            for document in &record.documents {
                if printed.insert(document.text.as_str()) {
                    writeln!(output, "\n--- {} ---\n{}", document.path, document.text)?;
                }
            }
            for license in &record.license_texts {
                if printed.insert(license.text.as_str()) {
                    let origin = match &license.origin {
                        unbill_license::TextOrigin::PackageFile { path } => path.as_str(),
                        unbill_license::TextOrigin::SpdxFallback => "standard SPDX text",
                    };
                    writeln!(
                        output,
                        "\n--- {} ({origin}) ---\n{}",
                        license.identifier, license.text
                    )?;
                }
            }
            writeln!(output)?;
        }
    }
    Ok(())
}
// sirno:witness:unbill-license:end
