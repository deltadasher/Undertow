// Application startup. cxx-qt-lib only offers QGuiApplication, but the tray icon
// needs a QApplication on Plasma (its status-notifier backend builds a QMenu),
// so this one function is C++. Everything else is Rust or QML.
#pragma once

/// Creates the application, loads Main.qml and runs the event loop until quit.
int undertow_run();
