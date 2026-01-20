#[cfg(test)]
mod ninja_runtime_integration_tests {
    use ninja::scripting::NinjaEngine;
    use std::io::Write;
    use std::{fs, path::Path};
    use tempfile::NamedTempFile;

    pub fn write_stub_script(dest: &Path) {
        let content = r#"function start()
            end

            function stop()
            end
            "#;
        fs::write(dest, content).expect("Failed to write stub script");
    }

    #[tokio::test]
    async fn test_engine_init_globals() {
        let engine = NinjaEngine::new().await.unwrap();
        let globals = engine.lua.globals();

        assert!(globals.contains_key("fs").unwrap());
        assert!(globals.contains_key("env").unwrap());
        assert!(globals.contains_key("shell").unwrap());
        assert!(globals.contains_key("time").unwrap());
        assert!(globals.contains_key("json").unwrap());
        assert!(globals.contains_key("http").unwrap());
        assert!(globals.contains_key("log").unwrap());
    }

    #[tokio::test]
    async fn test_execute_inline_script() {
        let engine = NinjaEngine::new().await.unwrap();
        assert!(engine.execute("x = 2 + 2", None).is_ok());
        assert!(engine.execute("error('fail')", None).is_err());
    }

    #[tokio::test]
    async fn test_execute_file() {
        let engine = NinjaEngine::new().await.unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "x = 123").unwrap();

        assert!(engine.execute_file(&tmp.path().to_path_buf(), None).is_ok());
    }

    #[tokio::test]
    async fn test_execute_function_from_returned_table() {
        let engine = NinjaEngine::new().await.unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "return {{ greet = function() print('hi') end }}").unwrap();

        let path = tmp.into_temp_path();
        assert!(
            engine
                .execute_function("greet", &path.to_path_buf(), None)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_execute_function_from_global() {
        let engine = NinjaEngine::new().await.unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "function greet() print('hi') end").unwrap();

        let path = tmp.into_temp_path();
        assert!(
            engine
                .execute_function("greet", &path.to_path_buf(), None)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_execute_function_with_return_value() {
        let engine = NinjaEngine::new().await.unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "function add() return 2 + 2 end").unwrap();

        let path = tmp.into_temp_path();
        // Function executes successfully even with return value
        assert!(
            engine
                .execute_function("add", &path.to_path_buf(), None)
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_execute_function_nonexistent() {
        let engine = NinjaEngine::new().await.unwrap();

        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "function greet() print('hi') end").unwrap();

        let path = tmp.into_temp_path();
        // Trying to execute a function that doesn't exist should fail
        assert!(
            engine
                .execute_function("nonexistent", &path.to_path_buf(), None)
                .is_err()
        );
    }

    #[tokio::test]
    async fn test_execute_inline_with_globals() {
        let engine = NinjaEngine::new().await.unwrap();
        
        // Test that global modules are accessible
        assert!(engine.execute("local x = fs", None).is_ok());
        assert!(engine.execute("local x = env", None).is_ok());
        assert!(engine.execute("local x = shell", None).is_ok());
        assert!(engine.execute("local x = time", None).is_ok());
    }
}

#[cfg(test)]
mod ninja_api_integration_tests {
    use crate::ninja_runtime_integration_tests::write_stub_script;
    use ninja::{
        manager::ShurikenManager,
        scripting::NinjaEngine,
        shuriken::{ManagementType, Shuriken, ShurikenMetadata, get_process_start_time},
        util::kill_process_by_pid,
    };
    use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
    use tempfile::tempdir;
    use tokio::sync::{Mutex, RwLock};

    #[tokio::test]
    async fn test_lockfile_written_for_script() {
        let dir = tempdir().unwrap();
        let lockfile = dir.path().join(".ninja").join("shuriken.lck");
        let engine = NinjaEngine::new().await.unwrap();
        let script_path = dir.path().join(".ninja").join("dummy.ns");
        fs::create_dir_all(&script_path.parent().unwrap()).unwrap();
        write_stub_script(&script_path);

        // fake script shuriken
        let shuriken = Shuriken {
            metadata: ShurikenMetadata {
                name: "test_script".into(),
                id: "id2".into(),
                version: "1.0.0".to_string(),
                management: Some(ManagementType::Script {
                    script_path: PathBuf::from("dummy.ns"),
                }),
                shuriken_type: "daemon".into(),
                require_admin: false,
            },
            config: None,
            logs: None,
            tools: None,
        };

        shuriken.start(Some(&engine), dir.path()).await.unwrap();
        assert!(lockfile.exists());
    }

    #[tokio::test]
    async fn test_manager_list_empty() {
        let dir = tempdir().unwrap();
        let engine = NinjaEngine::new().await.unwrap();
        let manager = ShurikenManager {
            root_path: dir.path().to_path_buf(),
            engine: Arc::new(Mutex::new(engine)),
            shurikens: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
        };

        let list = manager.list(false).await.unwrap();
        match list {
            either::Either::Right(v) => assert!(v.is_empty()),
            _ => panic!("Expected Right variant"),
        }
    }

    #[tokio::test]
    async fn test_kill_process_by_pid_invalid() {
        // 999999 should not exist
        #[cfg(windows)]
        let success = kill_process_by_pid(999999).unwrap();
        #[cfg(not(windows))]
        let success = kill_process_by_pid(999999);
        assert!(!success);
    }

    #[tokio::test]
    async fn test_get_process_start_time_invalid() {
        let result = get_process_start_time(999999);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stop_without_lockfile() {
        let engine = NinjaEngine::new().await.unwrap();
        let shuriken = Shuriken {
            metadata: ShurikenMetadata {
                name: "fake".into(),
                id: "id3".into(),
                version: "1.0.0".to_string(),
                management: Some(ManagementType::Script {
                    script_path: PathBuf::from("fake.lua"),
                }),
                shuriken_type: "daemon".into(),
                require_admin: false,
            },
            config: None,
            logs: None,
            tools: None,
        };

        // change dir to empty tempdir so lockfile isn't found
        let dir = tempdir().unwrap();

        let script_path = dir.path().join(".ninja").join("fake.lua");
        fs::create_dir_all(&script_path.parent().unwrap()).unwrap();
        write_stub_script(&script_path);

        let result = shuriken.stop(Some(&engine), dir.path()).await;
        assert!(result.is_err()); // script stop doesn't require pid
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let dir = tempdir().unwrap();
        let engine = NinjaEngine::new().await.unwrap();
        let manager = ShurikenManager {
            root_path: dir.path().to_path_buf(),
            engine: Arc::new(Mutex::new(engine)),
            shurikens: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
        };

        // Verify manager initialization
        assert_eq!(manager.root_path, dir.path());
        assert!(manager.shurikens.read().await.is_empty());
        assert!(manager.states.read().await.is_empty());
        assert!(manager.processes.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_shuriken_metadata_creation() {
        let metadata = ShurikenMetadata {
            name: "test".into(),
            id: "test-id".into(),
            version: "1.0.0".to_string(),
            management: None,
            shuriken_type: "daemon".into(),
            require_admin: false,
        };

        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.id, "test-id");
        assert_eq!(metadata.version, "1.0.0");
        assert!(!metadata.require_admin);
    }

    #[tokio::test]
    async fn test_lockfile_directory_creation() {
        let dir = tempdir().unwrap();
        let lockfile_dir = dir.path().join(".ninja");

        // Directory shouldn't exist yet
        assert!(!lockfile_dir.exists());

        // Create it
        fs::create_dir_all(&lockfile_dir).unwrap();

        // Now it should exist
        assert!(lockfile_dir.exists());
        assert!(lockfile_dir.is_dir());
    }
}
