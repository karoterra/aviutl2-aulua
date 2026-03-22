use std::fs;
use std::io::Read;
use zip::ZipArchive;

use aulua::config_loader::load_config;
use aulua::pack::pack_project;

#[test]
fn pack_project_creates_expected_archive_contents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    fs::write(
        root.join("src").join("main.in.anm2"),
        "-- version: ${PACKAGE_VERSION}\nprint('hello')\n",
    )
    .unwrap();

    fs::write(root.join("docs").join("README.md"), "# README\n").unwrap();

    fs::write(root.join("package-message.txt"), "line1\nline2\n").unwrap();

    fs::write(
        root.join("aulua.yaml"),
        r#"
project:
  variables: {}

build:
  out_dir: build

package:
  id: karoterra.example
  name: Example
  information: Example package
  version: 1.2.3
  out_dir: dist
  file_name: "{id}-v{version}.au2pkg.zip"
  script_sub_dir: "{id}"
  message:
    file: package-message.txt
  assets:
    - src: docs/README.md
      dest: Script/{id}/docs/README.md

scripts:
  - name: main.anm2
    sources:
      - path: src/main.in.anm2
"#,
    )
    .unwrap();

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let archive_path = pack_project(&config).unwrap();

    assert_eq!(
        archive_path.file_name().unwrap().to_string_lossy(),
        "karoterra.example-v1.2.3.au2pkg.zip"
    );

    let file = fs::File::open(&archive_path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();

    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();

    assert!(names.contains(&"package.ini".to_string()));
    assert!(names.contains(&"package.txt".to_string()));
    assert!(names.contains(&"Script/karoterra.example/main.anm2".to_string()));
    assert!(names.contains(&"Script/karoterra.example/docs/README.md".to_string()));

    let mut package_ini = String::new();
    zip.by_name("package.ini")
        .unwrap()
        .read_to_string(&mut package_ini)
        .unwrap();
    assert!(package_ini.contains("[package]\r\n"));
    assert!(package_ini.contains("id=karoterra.example\r\n"));
    assert!(package_ini.contains("name=Example\r\n"));
    assert!(package_ini.contains("information=Example package\r\n"));

    let mut package_txt = Vec::new();
    zip.by_name("package.txt")
        .unwrap()
        .read_to_end(&mut package_txt)
        .unwrap();
    assert_eq!(package_txt, b"line1\r\nline2\r\n");

    let mut script = String::new();
    zip.by_name("Script/karoterra.example/main.anm2")
        .unwrap()
        .read_to_string(&mut script)
        .unwrap();
    assert!(script.contains("-- version: 1.2.3"));

    let mut readme = String::new();
    zip.by_name("Script/karoterra.example/docs/README.md")
        .unwrap()
        .read_to_string(&mut readme)
        .unwrap();
    assert_eq!(readme, "# README\n");
}
