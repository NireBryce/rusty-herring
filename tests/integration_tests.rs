use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;
use rusty_herring::{App, Script, extract_description, scan_directory};

fn make_script(name: &str, category: Option<&str>) -> Script {
    Script {
        path: format!("/tmp/{}", name),
        name: name.to_string(),
        description: None,
        category: category.map(String::from),
    }
}

fn make_executable(path: &std::path::Path) {
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

mod app_tests {
    use super::*;

    #[test]
    fn new_initializes_with_defaults() {
        let scripts = vec![make_script("test.sh", None)];
        let app = App::new(scripts);

        assert_eq!(app.selected_index, 0);
        assert!(!app.should_quit);
        assert!(!app.viewing_output);
        assert!(!app.showing_help);
        assert!(app.output_text.is_empty());
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn next_increments_selection() {
        let scripts = vec![
            make_script("a.sh", None),
            make_script("b.sh", None),
            make_script("c.sh", None),
        ];
        let mut app = App::new(scripts);

        assert_eq!(app.selected_index, 0);
        app.next();
        assert_eq!(app.selected_index, 1);
        app.next();
        assert_eq!(app.selected_index, 2);
    }

    #[test]
    fn next_stops_at_end() {
        let scripts = vec![
            make_script("a.sh", None),
            make_script("b.sh", None),
        ];
        let mut app = App::new(scripts);

        app.next();
        app.next();
        app.next();
        assert_eq!(app.selected_index, 1);
    }

    #[test]
    fn previous_decrements_selection() {
        let scripts = vec![
            make_script("a.sh", None),
            make_script("b.sh", None),
        ];
        let mut app = App::new(scripts);
        app.selected_index = 1;

        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn previous_stops_at_zero() {
        let scripts = vec![make_script("a.sh", None)];
        let mut app = App::new(scripts);

        app.previous();
        app.previous();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn quit_sets_flag() {
        let mut app = App::new(vec![]);
        assert!(!app.should_quit);

        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn help_toggle() {
        let mut app = App::new(vec![]);

        app.show_help();
        assert!(app.showing_help);

        app.hide_help();
        assert!(!app.showing_help);
    }

    #[test]
    fn scroll_output() {
        let mut app = App::new(vec![]);
        app.output_scroll = 5;

        app.scroll_output_up();
        assert_eq!(app.output_scroll, 4);

        app.scroll_output_down(10);
        assert_eq!(app.output_scroll, 5);

        app.scroll_output_down(5);
        assert_eq!(app.output_scroll, 5);
    }

    #[test]
    fn scroll_output_up_stops_at_zero() {
        let mut app = App::new(vec![]);
        app.output_scroll = 1;

        app.scroll_output_up();
        assert_eq!(app.output_scroll, 0);

        app.scroll_output_up();
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn back_to_list_resets_state() {
        let mut app = App::new(vec![]);
        app.viewing_output = true;
        app.output_text = "some output".to_string();
        app.output_scroll = 5;

        app.back_to_list();

        assert!(!app.viewing_output);
        assert!(app.output_text.is_empty());
        assert_eq!(app.output_scroll, 0);
    }
}

mod extract_description_tests {
    use super::*;

    #[test]
    fn extracts_hash_comment() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "#!/bin/bash").unwrap();
        writeln!(file, "# This is a description").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("This is a description".to_string()));
    }

    #[test]
    fn extracts_double_slash_comment() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.js");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "// JavaScript description").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("JavaScript description".to_string()));
    }

    #[test]
    fn extracts_double_dash_comment() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sql");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "-- SQL description").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("SQL description".to_string()));
    }

    #[test]
    fn skips_empty_lines() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        let mut file = File::create(&path).unwrap();
        writeln!(file).unwrap();
        writeln!(file).unwrap();
        writeln!(file, "# After empty lines").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("After empty lines".to_string()));
    }

    #[test]
    fn skips_shebang() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "#!/usr/bin/env python3").unwrap();
        writeln!(file, "# Real description").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("Real description".to_string()));
    }

    #[test]
    fn returns_none_for_no_comment() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "echo 'hello'").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, None);
    }

    #[test]
    fn skips_empty_comments() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("script.sh");
        let mut file = File::create(&path).unwrap();
        writeln!(file, "#").unwrap();
        writeln!(file, "# Actual description").unwrap();

        let desc = extract_description(path.to_str().unwrap()).unwrap();
        assert_eq!(desc, Some("Actual description".to_string()));
    }
}

mod scan_directory_tests {
    use super::*;

    #[test]
    fn finds_executable_scripts() {
        let dir = TempDir::new().unwrap();
        let script_path = dir.path().join("test.sh");
        File::create(&script_path).unwrap();
        make_executable(&script_path);

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].name, "test.sh");
        assert_eq!(scripts[0].category, None);
    }

    #[test]
    fn ignores_non_executable_files() {
        let dir = TempDir::new().unwrap();
        let script_path = dir.path().join("readme.txt");
        File::create(&script_path).unwrap();

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert!(scripts.is_empty());
    }

    #[test]
    fn scans_subdirectories_with_category() {
        let dir = TempDir::new().unwrap();

        let subdir = dir.path().join("utils");
        fs::create_dir(&subdir).unwrap();
        let script_path = subdir.join("helper.sh");
        File::create(&script_path).unwrap();
        make_executable(&script_path);

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].name, "helper.sh");
        assert_eq!(scripts[0].category, Some("utils".to_string()));
    }

    #[test]
    fn handles_mixed_root_and_subdirectory_scripts() {
        let dir = TempDir::new().unwrap();

        let root_script = dir.path().join("root.sh");
        File::create(&root_script).unwrap();
        make_executable(&root_script);

        let subdir = dir.path().join("tools");
        fs::create_dir(&subdir).unwrap();
        let sub_script = subdir.join("tool.sh");
        File::create(&sub_script).unwrap();
        make_executable(&sub_script);

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert_eq!(scripts.len(), 2);

        let root = scripts.iter().find(|s| s.name == "root.sh").unwrap();
        assert_eq!(root.category, None);

        let tool = scripts.iter().find(|s| s.name == "tool.sh").unwrap();
        assert_eq!(tool.category, Some("tools".to_string()));
    }

    #[test]
    fn extracts_description_from_scripts() {
        let dir = TempDir::new().unwrap();
        let script_path = dir.path().join("described.sh");
        let mut file = File::create(&script_path).unwrap();
        writeln!(file, "#!/bin/bash").unwrap();
        writeln!(file, "# My awesome script").unwrap();
        drop(file);
        make_executable(&script_path);

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].description, Some("My awesome script".to_string()));
    }

    #[test]
    fn handles_empty_directory() {
        let dir = TempDir::new().unwrap();

        let scripts = scan_directory(dir.path().to_str().unwrap()).unwrap();

        assert!(scripts.is_empty());
    }
}
