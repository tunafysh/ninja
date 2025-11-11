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

        assert!(engine.execute_file(&tmp.path().to_path_buf()).is_ok());
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
    use ninja::{
        manager::ShurikenManager,
        scripting::NinjaEngine,
        shuriken::{
            MaintenanceType, Shuriken, ShurikenConfig, ShurikenMetadata, get_process_start_time,
            kill_process_by_pid,
        },
        types::FieldValue,
    };
    use std::{collections::HashMap, env, fs, path::PathBuf, sync::Arc};
    use tempfile::tempdir;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_configure_generates_file() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("out.conf");

        let mut fields = HashMap::new();
        fields.insert("key".into(), FieldValue::String("value".into()));

        let shuriken = Shuriken {
            metadata: ShurikenMetadata {
                name: "test".into(),
                id: "id1".into(),
                version: "1.0.0".to_string(),
                maintenance: MaintenanceType::Script {
                    script_path: PathBuf::from("dummy.lua"),
                },
                shuriken_type: "script".into(),
                add_path: false,
                require_admin: false,
            },
            config: Some(ShurikenConfig {
                config_path: config_path.clone(),
                options: Some(fields),
            }),
            logs: None,
        };

        let path = env::current_exe().unwrap();

        let result = shuriken.configure(path).await;
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[tokio::test]
    async fn test_lockfile_written_for_script() {
        let dir = tempdir().unwrap();
        let lockfile = dir.path().join("shuriken.lck");

        // fake script shuriken
        let shuriken = Shuriken {
            metadata: ShurikenMetadata {
                name: "test_script".into(),
                id: "id2".into(),
                version: "1.0.0".to_string(),
                maintenance: MaintenanceType::Script {
                    script_path: PathBuf::from("dummy.lua"),
                },
                shuriken_type: "script".into(),
                add_path: false,
                require_admin: false,
            },
            config: None,
            logs: None,
        };

        let result = shuriken.start().await;
        assert!(result.is_ok());
        assert!(lockfile.exists());
    }

    #[tokio::test]
    async fn test_manager_initializes_empty() {
        let dir = tempdir().unwrap();
        let exe_dir = dir.path().to_path_buf();
        std::env::set_current_dir(&exe_dir).unwrap();

        // place a fake binary for current_exe() resolution
        let fake_exe = exe_dir.join("ninja_bin");
        fs::write(&fake_exe, "").unwrap();

        // Patch env::current_exe with fake_exe (hack by setting PATH etc. in real tests)

        let manager = ShurikenManager::new().await.unwrap();
        assert!(manager.shurikens.read().await.is_empty());
        assert!(manager.states.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_manager_list_empty() {
        let dir = tempdir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let engine = NinjaEngine::new().unwrap();
        let manager = ShurikenManager {
            root_path: dir.path().to_path_buf(),
            engine,
            shurikens: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
        };

        let list = manager.list(false).await.unwrap();
        match list {
            either::Either::Right(v) => assert!(v.is_empty()),
            _ => panic!("Expected Right variant"),
        }
    }

    #[test]
    fn test_kill_process_by_pid_invalid() {
        // 999999 should not exist
        let success = kill_process_by_pid(999999);
        assert!(!success);
    }

    #[test]
    fn test_get_process_start_time_invalid() {
        let result = get_process_start_time(999999);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_stop_without_lockfile() {
        let shuriken = Shuriken {
            metadata: ShurikenMetadata {
                name: "fake".into(),
                id: "id3".into(),
                version: "1.0.0".to_string(),
                maintenance: MaintenanceType::Script {
                    script_path: PathBuf::from("fake.lua"),
                },
                shuriken_type: "script".into(),
                add_path: false,
                require_admin: false,
            },
            config: None,
            logs: None,
        };

        // change dir to empty tempdir so lockfile isn't found
        let dir = tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = shuriken.stop().await;
        assert!(result.is_ok()); // script stop doesn't require pid

        std::env::set_current_dir(orig).unwrap();
    }
}
