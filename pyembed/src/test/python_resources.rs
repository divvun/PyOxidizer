// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use {
    crate::OxidizedPythonInterpreterConfig,
    anyhow::{anyhow, Result},
    python_oxidized_importer::{PackedResourcesSource, PyTempDir, PythonResourcesState},
    python_packed_resources::data::Resource,
    rusty_fork::rusty_fork_test,
    std::convert::TryFrom,
};

#[test]
fn multiple_resource_blobs() -> Result<()> {
    let mut state0 = PythonResourcesState::default();
    state0
        .add_resource(Resource {
            name: "foo".into(),
            is_python_module: true,
            in_memory_source: Some(vec![42].into()),
            ..Default::default()
        })
        .unwrap();
    let data0 = state0.serialize_resources(true, true)?;

    let mut state1 = PythonResourcesState::default();
    state1
        .add_resource(Resource {
            name: "bar".into(),
            is_python_module: true,
            in_memory_source: Some(vec![42, 42].into()),
            ..Default::default()
        })
        .unwrap();
    let data1 = state1.serialize_resources(true, true)?;

    let config = OxidizedPythonInterpreterConfig::default().resolve()?;

    let mut resources = PythonResourcesState::try_from(&config)?;
    resources.index_data(&data0).unwrap();
    resources.index_data(&data1).unwrap();

    assert!(resources.resources.contains_key("foo".into()));
    assert!(resources.resources.contains_key("bar".into()));

    Ok(())
}

#[test]
fn test_memory_mapped_file_resources() -> Result<()> {
    let current_dir = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow!("unable to find current exe parent"))?
        .to_path_buf();

    let mut state0 = PythonResourcesState::default();
    state0
        .add_resource(Resource {
            name: "foo".into(),
            is_python_module: true,
            in_memory_source: Some(vec![42].into()),
            ..Default::default()
        })
        .unwrap();
    let data0 = state0.serialize_resources(true, true)?;

    let resources_dir = current_dir.join("resources");
    if !resources_dir.exists() {
        std::fs::create_dir(&resources_dir)?;
    }

    let resources_path = resources_dir.join("test_memory_mapped_file_resources");
    std::fs::write(&resources_path, &data0)?;

    // Absolute path should work.
    let mut config = OxidizedPythonInterpreterConfig::default();
    config
        .packed_resources
        .push(PackedResourcesSource::MemoryMappedPath(
            resources_path.clone(),
        ));

    let resolved = config.clone().resolve()?;
    let resources = PythonResourcesState::try_from(&resolved)?;

    assert!(resources.resources.contains_key("foo".into()));

    // Now let's try with relative paths.
    let relative_path = pathdiff::diff_paths(&resources_path, std::env::current_dir()?).unwrap();
    config.packed_resources.clear();
    config
        .packed_resources
        .push(PackedResourcesSource::MemoryMappedPath(relative_path));

    let resolved = config.resolve()?;
    let resources = PythonResourcesState::try_from(&resolved)?;
    assert!(resources.resources.contains_key("foo".into()));

    Ok(())
}

fn get_interpreter<'interp, 'rsrc>() -> crate::MainPythonInterpreter<'interp, 'rsrc> {
    let mut config = crate::OxidizedPythonInterpreterConfig::default();
    config.interpreter_config.parse_argv = Some(false);
    config.set_missing_path_configuration = false;
    let interp = crate::MainPythonInterpreter::new(config).unwrap();

    interp
}

rusty_fork_test! {
    #[test]
    fn py_temp_dir_lifetimes() {
        let path = {
            let mut interp = get_interpreter();
            let py = interp.acquire_gil();
            let temp_dir = PyTempDir::new(py).unwrap();
            drop(py); // PyTempDir::drop reacquires the GIL for itself
            assert!(temp_dir.path().is_dir());
            temp_dir.path().to_path_buf()
        };
        assert!(!path.is_dir());
    }
}