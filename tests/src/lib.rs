#[cfg(test)]
mod ninja_runtime_integration_tests {
    use ninja::scripting::NinjaEngine;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_engine_init_globals() {
        let engine = NinjaEngine::new().unwrap();
        let globals = engine.lua.globals();

        assert!(globals.contains_key("fs").unwrap());
        assert!(globals.contains_key("env").unwrap());
        assert!(globals.contains_key("shell").unwrap());
        assert!(globals.contains_key("time").unwrap());
        assert!(globals.contains_key("json").unwrap());
        assert!(globals.contains_key("http").unwrap());
        assert!(globals.contains_key("log").unwrap());
    }

    #[test]
    fn test_execute_inline_script() {
        let engine = NinjaEngine::new().unwrap();
        assert!(engine.execute("x = 2 + 2").is_ok());
        assert!(engine.execute("error('fail')").is_err());
    }

    #[test]
    fn test_execute_file() {
        let engine = NinjaEngine::new().unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "x = 123").unwrap();

        assert!(engine.execute_file(tmp.path().to_str().unwrap()).is_ok());
    }

    #[test]
    fn test_execute_function_from_returned_table() {
        let engine = NinjaEngine::new().unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "return {{ greet = function() print('hi') end }}").unwrap();

        let path = tmp.into_temp_path();
        assert!(
            engine
                .execute_function("greet", &path.to_path_buf())
                .is_ok()
        );
    }

    #[test]
    fn test_execute_function_from_global() {
        let engine = NinjaEngine::new().unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "function greet() print('hi') end").unwrap();

        let path = tmp.into_temp_path();
        assert!(
            engine
                .execute_function("greet", &path.to_path_buf())
                .is_ok()
        );
    }
}

#[cfg(test)]
mod ninja_api_integration_tests {
    // use std::fs;
    // use ninja::manager::ShurikenManager;
    // use either;
    // use tempfile::tempdir;

    #[tokio::test]
    async fn test_load_shuriken_from_manifest() {
        // let dir = tempdir().unwrap();
        // let shuriken_dir = dir.path().join("shurikens/shadow-strike/.ninja");
        // fs::create_dir_all(&shuriken_dir).unwrap();

        // // Write manifest.toml
        // let manifest = r#"
        //     [shuriken]
        //     name = "Shadow Strike"
        //     id = "shadow-strike"
        //     type = "native"
        //     add-path = false

        //     [shuriken.maintenance]
        //     type = "native"
        //     bin-path = "echo"
        //     config-path = ""
        //     args = ["Hello", "World"]
        // "#;
        // fs::write(shuriken_dir.join("manifest.toml"), manifest).unwrap();

        // // Point ShurikenManager to our fake exe_dir
        // std::env::set_current_dir(dir.path()).unwrap();

        // let manager = ShurikenManager::new().await.unwrap();

        // // Should detect "shadow-strike"
        // let list = manager.list(false).await.unwrap();
        // let names = match list {
        //     either::Either::Right(names) => names,
        //     _ => panic!("Expected names"),
        // };

        // println!("{:#?}-{:#?}", names, dir);

        // assert!(names.contains(&"shadow-strike".to_string()));
    }
}
