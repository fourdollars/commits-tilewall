use std::process::Command;
use image::{ImageBuffer, Rgba};
use chrono::{NaiveDate, Datelike, Month};
use imageproc::drawing::draw_text_mut;
use rusttype::{Font, Scale};
use std::fs::File;
use std::io::Read;
use fontconfig::Fontconfig;

#[derive(Debug)]
struct Theme {
    background: Rgba<u8>,
    text_primary: Rgba<u8>,
    text_secondary: Rgba<u8>,
    separator: Rgba<u8>,
    commit_colors: [Rgba<u8>; 6],  // [no_commit, 1, 2-4, 5-9, 10-19, 20+]
}

impl Theme {
    fn dark() -> Self {
        Theme {
            background: Rgba([30, 30, 30, 255]),
            text_primary: Rgba([255, 255, 255, 255]),
            text_secondary: Rgba([200, 200, 200, 255]),
            separator: Rgba([70, 70, 70, 255]),
            commit_colors: [
                Rgba([50, 50, 50, 255]),      // no commits
                Rgba([40, 160, 40, 255]),     // 1 commit - brighter to be visible
                Rgba([60, 200, 60, 255]),     // 2-4 commits
                Rgba([80, 240, 80, 255]),     // 5-9 commits
                Rgba([120, 255, 120, 255]),   // 10-19 commits
                Rgba([160, 255, 160, 255]),   // 20+ commits
            ],
        }
    }

    fn light() -> Self {
        Theme {
            background: Rgba([255, 255, 255, 255]),
            text_primary: Rgba([50, 50, 50, 255]),
            text_secondary: Rgba([100, 100, 100, 255]),
            separator: Rgba([220, 220, 220, 255]),
            commit_colors: [
                Rgba([240, 240, 240, 255]),   // no commits
                Rgba([140, 240, 140, 255]),   // 1 commit - more distinct
                Rgba([100, 220, 100, 255]),   // 2-4 commits
                Rgba([60, 200, 60, 255]),     // 5-9 commits
                Rgba([40, 180, 40, 255]),     // 10-19 commits
                Rgba([20, 160, 20, 255]),     // 20+ commits
            ],
        }
    }

    fn github() -> Self {
        Theme {
            background: Rgba([255, 255, 255, 255]),
            text_primary: Rgba([24, 23, 23, 255]),
            text_secondary: Rgba([87, 96, 106, 255]),
            separator: Rgba([235, 237, 240, 255]),
            commit_colors: [
                Rgba([235, 237, 240, 255]),   // no commits
                Rgba([155, 233, 168, 255]),   // 1 commit - GitHub's actual color
                Rgba([100, 220, 123, 255]),   // 2-4 commits
                Rgba([64, 196, 99, 255]),     // 5-9 commits
                Rgba([48, 161, 78, 255]),     // 10-19 commits
                Rgba([33, 110, 57, 255]),     // 20+ commits
            ],
        }
    }
}

fn load_system_font() -> Font<'static> {
    let fontconfig = Fontconfig::new().unwrap();
    let font = fontconfig
        .find("sans-bold", None)
        .or_else(|| fontconfig.find("sans", None))
        .expect("Could not find a sans font on the system");
    
    let font_path = font.path;
    
    let mut font_file = File::open(font_path)
        .expect("Failed to open font file");
    let mut font_data = Vec::new();
    font_file.read_to_end(&mut font_data)
        .expect("Failed to read font file");
    
    Font::try_from_vec(font_data)
        .expect("Failed to load font")
}

fn get_commit_color(commit_count: i32, theme: &Theme) -> Rgba<u8> {
    let colors = &theme.commit_colors;
    match commit_count {
        0 => colors[0],
        1 => colors[1],
        2..=4 => colors[2],
        5..=9 => colors[3],
        10..=19 => colors[4],
        _ => colors[5],
    }
}

fn draw_sharp_text(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, text: &str, x: i32, y: i32, size: f32, color: Rgba<u8>, font: &Font) {
    let scale = Scale {
        x: size,
        y: size,
    };
    
    draw_text_mut(
        img,
        color,
        x,
        y,
        scale,
        font,
        text
    );
}

fn draw_block(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, size: u32, color: Rgba<u8>) {
    for by in 0..size {
        for bx in 0..size {
            let pixel_x = x + bx;
            let pixel_y = y + by;
            
            if pixel_x < img.width() && pixel_y < img.height() {
                img.put_pixel(pixel_x, pixel_y, color);
            }
        }
    }
}

fn generate_commit_image(author: &str, repos: &[String], theme_name: &str) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let theme = match theme_name.to_lowercase().as_str() {
        "dark" => Theme::dark(),
        "github" => Theme::github(),
        _ => Theme::light(),  // default to light theme
    };

    let block_size: u32 = 10;
    let space_size: u32 = 2;
    let year_spacing: u32 = 20;
    let month_grid_width: u32 = 4;  // 4 columns per month
    let month_grid_height: u32 = 8;  // 8 rows per month (to fit 31 days)
    let month_label_height: u32 = block_size * 2;  // Scale with block size
    let year_height: u32 = month_grid_height * (block_size + space_size) + month_label_height;
    let year_label_width: u32 = block_size * 5;  // Scale with block size
    let summary_width: u32 = block_size * 45;  // Increased width further
    let month_spacing: u32 = space_size * 3;  // Additional spacing between months

    // Load system font
    let font = load_system_font();
    
    // Collect commit dates and stats at the start
    let mut commit_dates: Vec<NaiveDate> = Vec::new();
    let mut commit_stats = std::collections::HashMap::new();

    for repo in repos {
        // Collect dates
        let output = Command::new("git")
            .arg("log")
            .arg("--author")
            .arg(author)
            .arg("--pretty=format:%cd")
            .arg("--date=short")
            .current_dir(repo)
            .output()
            .expect("Failed to execute git command");

        let commits = String::from_utf8_lossy(&output.stdout);
        for line in commits.lines() {
            if let Ok(date) = NaiveDate::parse_from_str(line, "%Y-%m-%d") {
                commit_dates.push(date);
            }
        }

        // Collect stats
        let stats_output = Command::new("git")
            .args(&[
                "log",
                "--author", author,
                "--pretty=format:%cd",
                "--date=short",
                "--numstat",
            ])
            .current_dir(repo)
            .output()
            .expect("Failed to execute git command");

        let stats = String::from_utf8_lossy(&stats_output.stdout);
        let mut current_date: Option<NaiveDate> = None;
        
        for line in stats.lines() {
            if let Ok(date) = NaiveDate::parse_from_str(line, "%Y-%m-%d") {
                current_date = Some(date);
            } else if let Some(date) = current_date {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 3 && parts[0] != "-" && parts[1] != "-" {
                    if let (Ok(added), Ok(deleted)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                        let entry = commit_stats.entry(date).or_insert((0, 0, 0));
                        entry.0 += 1;           // files
                        entry.1 += added;       // additions
                        entry.2 += deleted;     // deletions
                    }
                }
            }
        }
    }

    // Create a map to count commits per day
    let mut commit_count_per_day = std::collections::HashMap::new();
    for date in commit_dates {
        *commit_count_per_day.entry(date).or_insert(0) += 1;
    }

    // Find years that have commits and count commits per year
    let mut year_commit_counts: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    for (date, count) in &commit_count_per_day {
        *year_commit_counts.entry(date.year()).or_insert(0) += count;
    }

    // Get years with significant activity (more than 5 commits)
    let min_commits = 5;
    let mut active_years: Vec<i32> = year_commit_counts
        .iter()
        .filter(|&(_, count)| *count >= min_commits)
        .map(|(year, _)| *year)
        .collect();
    active_years.sort_unstable_by(|a, b| b.cmp(a));  // Sort in descending order

    println!("Found commits in years: {:?}", active_years);
    println!("Commit counts per year: {:?}", 
        active_years.iter()
            .map(|year| (year, year_commit_counts.get(year).unwrap_or(&0)))
            .collect::<Vec<_>>());

    // If no commits found, return a minimal image
    if active_years.is_empty() {
        println!("No commits found!");
        return ImageBuffer::new(1, 1);
    }

    // Calculate image dimensions based on active years only
    let years_count = active_years.len() as u32;
    let width = year_label_width + 
                12 * (month_grid_width * (block_size + space_size) + month_spacing) + 
                summary_width + 
                space_size * 4;  // Extra padding
    let height = (year_height + year_spacing) * years_count;
    
    let mut img = ImageBuffer::new(width, height);

    // Fill background
    for pixel in img.pixels_mut() {
        *pixel = theme.background;
    }

    // Fill the image based on commit counts
    for (year_index, &year) in active_years.iter().enumerate() {
        let year_offset = (year_index as u32) * (year_height + year_spacing);
        
        // Draw year text in dark color
        let year_text = year.to_string();
        draw_sharp_text(
            &mut img,
            &year_text,
            5,
            (year_offset + (year_height / 2)) as i32 - (block_size as i32 / 2),
            block_size as f32 * 1.6,
            theme.text_primary,
            &font
        );

        // Process each month
        for month in 1..=12 {
            let month_x_offset = year_label_width + 
                                (month - 1) as u32 * (month_grid_width * (block_size + space_size) + month_spacing);

            // Draw month abbreviation in dark color
            if let Some(month_name) = Month::try_from(month as u8).ok() {
                let month_abbr = month_name.name().chars().take(3).collect::<String>();
                draw_sharp_text(
                    &mut img,
                    &month_abbr,
                    month_x_offset as i32,
                    year_offset as i32,
                    block_size as f32 * 1.2,
                    theme.text_secondary,
                    &font
                );
            }

            // Draw all days in a grid
            let days_in_month = match month {
                1 => 31, // January
                2 => if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) { 29 } else { 28 }, // February
                3 => 31, // March
                4 => 30, // April
                5 => 31, // May
                6 => 30, // June
                7 => 31, // July
                8 => 31, // August
                9 => 30, // September
                10 => 31, // October
                11 => 30, // November
                12 => 31, // December
                _ => 0, // Invalid month
            };

            for day in 1..=days_in_month {  // Adjusted to use days_in_month
                let col = (day - 1) % month_grid_width;
                let row = (day - 1) / month_grid_width;

                // Only draw if within bounds
                if row < month_grid_height && day <= days_in_month {  // Ensure we only draw within the grid height and valid days
                    let x = month_x_offset + col * (block_size + space_size);
                    let y = year_offset + month_label_height + row * (block_size + space_size);

                    // Set color based on number of commits
                    let color_value = if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                        if let Some(&count) = commit_count_per_day.get(&date) {
                            get_commit_color(count, &theme)
                        } else {
                            get_commit_color(0, &theme)
                        }
                    } else {
                        theme.commit_colors[0]  // Use no-commit color for invalid dates
                    };

                    // Draw the block
                    for by in 0..block_size {
                        for bx in 0..block_size {
                            let pixel_x = x + bx;
                            let pixel_y = y + by;

                            if pixel_x < img.width() && pixel_y < img.height() {
                                img.put_pixel(pixel_x, pixel_y, color_value);
                            }
                        }
                    }
                }
            }
        }

        // Draw year separator line in light gray
        if year_index > 0 {
            for x in 0..width {
                let line_y = year_offset - (year_spacing / 2);
                img.put_pixel(x, line_y, theme.separator);  // Light gray line
            }
        }

        // Draw summary on the right side
        let summary_x = width - summary_width - space_size * 2;
        let stats_x = summary_x;  // Stats start at the same x position
        let legend_x = summary_x + block_size * 20;  // Color legend starts after stats

        // Count total commits and stats for the year
        let year_total: i32 = commit_count_per_day.iter()
            .filter(|(date, _)| date.year() == year)
            .map(|(_, &count)| count)
            .sum();

        let year_stats = commit_stats.iter()
            .filter(|(date, _)| date.year() == year)
            .fold((0, 0, 0), |acc, (_, &(files, added, deleted))| {
                (acc.0 + files, acc.1 + added, acc.2 + deleted)
            });

        // Calculate commit level counts
        let level_counts = [
            commit_count_per_day.iter()
                .filter(|(date, &count)| date.year() == year && count == 1)
                .count(),
            commit_count_per_day.iter()
                .filter(|(date, &count)| date.year() == year && (2..=4).contains(&count))
                .count(),
            commit_count_per_day.iter()
                .filter(|(date, &count)| date.year() == year && (5..=9).contains(&count))
                .count(),
            commit_count_per_day.iter()
                .filter(|(date, &count)| date.year() == year && (10..=19).contains(&count))
                .count(),
            commit_count_per_day.iter()
                .filter(|(date, &count)| date.year() == year && count >= 20)
                .count(),
        ];

        // Draw summary text with stats
        let summary_lines = [
            format!("{} commits total", year_total),
            format!("{} files changed", year_stats.0),
            format!("{} insertions(+)", year_stats.1),
            format!("{} deletions(-)", year_stats.2),
        ];

        for (i, text) in summary_lines.iter().enumerate() {
            draw_sharp_text(
                &mut img,
                text,
                stats_x as i32,
                (year_offset + block_size + i as u32 * (block_size + space_size)) as i32,
                block_size as f32 * 0.8,
                theme.text_primary,
                &font
            );
        }

        // Draw commit level counts (adjusted position to account for stats)
        for (i, &count) in level_counts.iter().enumerate() {
            if count > 0 && 
               legend_x + block_size <= width && 
               year_offset + (i as u32 + 7) * (block_size + space_size) + block_size <= height {
                
                // Draw colored square
                draw_block(
                    &mut img,
                    legend_x,
                    year_offset + (i as u32 + 7) * (block_size + space_size),
                    block_size,
                    theme.commit_colors[i + 1]
                );

                // Draw count text
                let level_text = match i {
                    0 => format!("{} days with 1 commit", count),
                    1 => format!("{} days with 2-4 commits", count),
                    2 => format!("{} days with 5-9 commits", count),
                    3 => format!("{} days with 10-19 commits", count),
                    _ => format!("{} days with 20+ commits", count),
                };

                // Draw text only if there's enough space
                let text_x = legend_x + block_size + space_size * 2;
                if text_x + block_size * 15 <= width {
                    draw_sharp_text(
                        &mut img,
                        &level_text,
                        text_x as i32,
                        ((i as u32 + 7) * (block_size + space_size)) as i32,
                        block_size as f32 * 0.8,
                        theme.text_secondary,
                        &font
                    );
                }
            }
        }
    }

    img
}

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <author> <repo1> [repo2...] [--theme <theme>]", args[0]);
        eprintln!("Available themes: light (default), dark, github");
        std::process::exit(1);
    }

    let author = &args[1];
    let mut repos = Vec::new();
    let mut theme = "light";

    let mut i = 2;
    while i < args.len() {
        if args[i] == "--theme" && i + 1 < args.len() {
            theme = &args[i + 1];
            i += 2;
        } else {
            repos.push(args[i].clone());
            i += 1;
        }
    }

    let img = generate_commit_image(author, &repos, theme);
    let output_path = format!("commit_image_{}.png", author.replace(" ", "_"));
    img.save(output_path).expect("Failed to save the image");
}
