#include "shim.h"

#include <QApplication>
#include <QIcon>
#include <QQmlApplicationEngine>
#include <QUrl>
#include <QtWebEngineQuick>

int undertow_run()
{
    static int argc = 1;
    static char name[] = "undertow";
    static char *argv[] = {name, nullptr};

    QCoreApplication::setOrganizationName(QStringLiteral("undertow"));
    QCoreApplication::setApplicationName(QStringLiteral("undertow"));
    // Shares the GPU context between Qt Quick and Chromium; has to precede the application object.
    QtWebEngineQuick::initialize();

    QApplication app(argc, argv);
    // The daemon lives in the tray; closing the picker must not end it.
    QApplication::setQuitOnLastWindowClosed(false);
    QGuiApplication::setDesktopFileName(QStringLiteral("undertow"));
    QGuiApplication::setWindowIcon(QIcon(QStringLiteral(":/qt/qml/dev/undertow/assets/undertow.svg")));

    QQmlApplicationEngine engine;
    engine.load(QUrl(QStringLiteral("qrc:/qt/qml/dev/undertow/qml/Main.qml")));
    if (engine.rootObjects().isEmpty())
        return 1;
    return app.exec();
}
