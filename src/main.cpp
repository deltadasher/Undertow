#include "daemon.h"
#include "library.h"

#include <QApplication>
#include <QCoreApplication>
#include <QFileInfo>
#include <QLocalSocket>
#include <QProcess>
#include <QSettings>
#include <QTextStream>
#include <QThread>

namespace {

const char *kUsage = R"(undertow - interactive web wallpapers for Wayland (niri, Hyprland, Plasma, sway)

  undertow list                    show installed wallpapers
  undertow set <id|https://url>    switch wallpaper (starts the daemon if needed)
  undertow add <source>            install from GitHub or link a local directory
                                     owner/repo   owner/repo/sub/dir   owner/repo@branch
                                     https://github.com/owner/repo[/tree/ref/sub/dir]
  undertow update [id]             git pull one wallpaper, or all of them
  undertow remove <id>             delete an installed wallpaper
  undertow props                   show the current wallpaper's settings
  undertow prop <name> <value>     change a setting live
  undertow reload | status | quit  talk to the running daemon
  undertow run                     run the daemon in the foreground (for autostart)
)";

// Sends one command to the daemon. Returns false if no daemon is listening.
bool send(const QString &line, QString *reply)
{
    QLocalSocket sock;
    sock.connectToServer(Daemon::socketName());
    if (!sock.waitForConnected(500))
        return false;
    sock.write(line.toUtf8() + '\n');
    sock.flush();
    QByteArray data;
    while (sock.state() == QLocalSocket::ConnectedState && sock.waitForReadyRead(60000))
        data += sock.readAll();
    data += sock.readAll();
    *reply = QString::fromUtf8(data);
    return true;
}

int runDaemon(int argc, char **argv)
{
    if (qEnvironmentVariableIsEmpty("QT_QPA_PLATFORM"))
        qputenv("QT_QPA_PLATFORM", "wayland");
    Daemon::registerScheme();
    QApplication app(argc, argv);
    QTextStream err(stderr);
    if (!QGuiApplication::platformName().startsWith("wayland")) {
        err << "undertow: needs a Wayland session with wlr-layer-shell (niri, Hyprland, Plasma, sway)\n";
        return 1;
    }
    app.setQuitOnLastWindowClosed(false);

    Daemon daemon;
    if (!daemon.listen()) {
        err << "undertow: already running (undertow quit to stop it)\n";
        return 1;
    }
    const QString target = QSettings().value("current", "snow").toString();
    const QString error = daemon.apply(target);
    if (!error.isEmpty()) {
        err << "undertow: " << error << "; falling back to snow\n";
        err.flush();
        daemon.apply(QStringLiteral("snow"));
    }
    return app.exec();
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication::setOrganizationName(QStringLiteral("undertow"));
    QCoreApplication::setApplicationName(QStringLiteral("undertow"));

    const QString cmd = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QStringLiteral("help");
    if (cmd == "run")
        return runDaemon(argc, argv);

    QCoreApplication app(argc, argv);
    QTextStream out(stdout), err(stderr);
    QStringList args = app.arguments().mid(2);
    const QString arg = args.join(' ');

    if (cmd == "help" || cmd == "-h" || cmd == "--help") {
        out << kUsage;
        return 0;
    }
    if (cmd == "list") {
        const QString current = QSettings().value("current").toString();
        for (const Wallpaper &w : Library::all()) {
            const char *kind = w.builtin ? "preset" : QFileInfo::exists(w.dir + "/.git") ? "git" : "local";
            out << (w.id == current ? "* " : "  ") << w.id.leftJustified(24) << " " << QString(kind).leftJustified(7)
                << " " << w.title << "\n";
        }
        return 0;
    }
    if (cmd == "add")
        return arg.isEmpty() ? (err << kUsage, 1) : Library::add(arg);
    if (cmd == "update")
        return Library::update(arg);
    if (cmd == "remove")
        return Library::remove(arg);

    if (cmd == "set") {
        const bool isUrl = arg.startsWith("http://") || arg.startsWith("https://");
        if (arg.isEmpty() || (!isUrl && !Library::find(arg))) {
            err << "undertow: no wallpaper '" << arg << "' (see: undertow list)\n";
            return 1;
        }
        QString reply;
        if (send("set " + arg, &reply)) {
            out << reply;
            return reply.startsWith("error") ? 1 : 0;
        }
        // No daemon yet: remember the choice and start one.
        QSettings().setValue("current", arg);
        QSettings().sync();
        if (!QProcess::startDetached(QCoreApplication::applicationFilePath(), {"run"})) {
            err << "undertow: could not start the daemon\n";
            return 1;
        }
        out << "started undertow with " << arg << "\n";
        return 0;
    }

    if (cmd == "prop" || cmd == "props" || cmd == "reload" || cmd == "status" || cmd == "quit") {
        QString reply;
        if (!send(arg.isEmpty() ? cmd : cmd + " " + arg, &reply)) {
            err << "undertow: not running (undertow set <id> starts it)\n";
            return 1;
        }
        out << reply;
        return reply.startsWith("error") ? 1 : 0;
    }

    err << "undertow: unknown command '" << cmd << "'\n\n" << kUsage;
    return 1;
}
