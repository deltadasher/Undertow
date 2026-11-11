use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("dev.undertow").qml_files(["qml/Main.qml", "qml/WallWindow.qml", "qml/Picker.qml", "qml/PropEditor.qml"]),
    )
    .file("src/backend.rs")
    .cpp_file("cpp/shim.cpp")
    .include_dir("cpp")
    .qrc_resources(["assets/undertow.svg"])
    .qt_module("Gui")
    .qt_module("Widgets")
    .qt_module("Quick")
    .qt_module("WebEngineQuick")
    .build();
}
