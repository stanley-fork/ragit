
use crate::error::Error;
use ragit_fs::{
    WriteMode,
    basename,
    create_dir_all,
    exists,
    file_name,
    initialize_log,
    join,
    read_dir,
    read_string,
    remove_file,
    set_extension,
    write_log,
    write_string,
};
use serde::Serialize;

pub fn init_server() -> Result<(), Error> {
    initialize_log(
        Some(String::from("ragit-server-logs")),
        false,  // dump_to_stdout
        false,  // dump_to_stderr
        true,   // keep_previous_file
    )?;
    write_log("server", "hello from ragithub!");
    let grass_option = grass::Options::default();
    let tera_context = get_colors()?;
    let mut tmp_files = vec![];

    if !exists("./static") {
        create_dir_all("./static")?;
    }

    for scss_tera in read_dir("./styles", false)?.iter() {
        if scss_tera.ends_with(".scss.tera") {
            let c = read_string(&scss_tera)?;
            let t = tera::Tera::one_off(&c, &tera_context, false)?;
            let scss = scss_tera.get(..(scss_tera.len() - 5)).unwrap();
            tmp_files.push(scss.to_string());

            write_string(
                scss,
                &t,
                WriteMode::CreateOrTruncate,
            )?;
        }
    }

    for scss in read_dir("./styles", false)?.iter() {
        if scss.ends_with(".scss") {
            let css = grass::from_path(&scss, &grass_option)?;
            write_string(
                &join(
                    "./static",
                    &set_extension(&file_name(&scss)?, "css")?,
                )?,
                &css,
                WriteMode::CreateOrTruncate,
            )?;
        }
    }

    for tmp_file in tmp_files.iter() {
        remove_file(tmp_file)?;
    }

    for script in read_dir("./scripts", false)?.iter() {
        write_string(
            &join(
                "./static",
                &basename(script)?,
            )?,
            &read_string(script)?,
            WriteMode::CreateOrTruncate,
        )?;
    }

    for component in read_dir("./components", false)?.iter() {
        write_string(
            &join(
                "./static",
                &basename(component)?,
            )?,
            &read_string(component)?,
            WriteMode::CreateOrTruncate,
        )?;
    }

    Ok(())
}

#[derive(Serialize)]
struct Color {
    basic: bool,
    name: String,
    hex: String,
}

fn get_colors() -> Result<tera::Context, Error> {
    let raw_colors = vec![
        crate::colors::COLORS.iter().map(
            |(name, value)| (
                name.to_string(),
                *value,
            )
        ).collect::<Vec<_>>(),
        crate::colors::COLORS.iter().map(
            |(name, (r, g, b))| (
                format!("{name}-compl"),
                (255 - r, 255 - g, 255 - b),
            )
        ).collect(),
    ].concat();

    let mut colors = vec![];

    for (name, (r, g, b)) in raw_colors {
        colors.push(Color {
            basic: !name.contains("-compl"),
            name: name.to_string(),
            hex: format!("#{r:02x}{g:02x}{b:02x}"),
        });
        colors.push(Color {
            basic: false,
            name: format!("{name}-trans"),
            hex: format!("#{r:02x}{g:02x}{b:02x}80"),
        });
    }

    let mut result = tera::Context::new();
    result.insert("colors", &colors);
    Ok(result)
}
