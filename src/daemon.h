#pragma once
#include "library.h"
#include "sysstats.h"

#include <QHash>
#include <QJsonValue>
#include <QLocalServer>
#include <QObject>
#include <QUrl>

class QScreen;
class QWebEngineProfile;
class QWebEngineView;

// Owns one background-layer web view per screen, feeds them stats and
// properties, and listens on a local socket for commands from the CLI.
class Daemon : public QObject
{
    Q_OBJECT
public:
    explicit Daemon(QObject *parent = nullptr);
    ~Daemon() override;

    static void registerScheme(); // must run before QApplication exists
    static QString socketName();

    bool listen();
    QString apply(const QString &target); // returns an error message, empty on success

private:
    void addScreen(QScreen *screen);
    void removeScreen(QScreen *screen);
    QString handle(const QString &line);
    QJsonObject propValues() const;
    void pushProps(QWebEngineView *view);
    void emitJs(const QString &event, const QJsonValue &detail, QWebEngineView *only = nullptr);

    QWebEngineProfile *m_profile = nullptr;
    QHash<QScreen *, QWebEngineView *> m_views;
    QLocalServer m_server;
    SysStats m_stats;
    Wallpaper m_current;
    QUrl m_url;
};
