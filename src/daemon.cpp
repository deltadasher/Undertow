#include "daemon.h"

#include <LayerShellQt/Window>
#include <QCoreApplication>
#include <QEvent>
#include <QPointer>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLocalSocket>
#include <QMimeDatabase>
#include <QScreen>
#include <QSettings>
#include <QTimer>
#include <QWebEnginePage>
#include <QWebEngineProfile>
#include <QWebEngineScript>
#include <QWebEngineScriptCollection>
#include <QWebEngineSettings>
#include <QWebEngineUrlRequestJob>
#include <QWebEngineUrlScheme>
#include <QWebEngineUrlSchemeHandler>
#include <QWebEngineView>
#include <QWindow>
#include <unistd.h>

namespace {

const QByteArray kScheme = "undertow";

// Serves undertow://<wallpaper-id>/<path> from the wallpaper's directory.
// A real scheme (rather than file://) lets pages use fetch() and ES modules.
class SchemeHandler : public QWebEngineUrlSchemeHandler
{
public:
    using QWebEngineUrlSchemeHandler::QWebEngineUrlSchemeHandler;

    void requestStarted(QWebEngineUrlRequestJob *job) override
    {
        const QUrl url = job->requestUrl();
        const auto wp = Library::find(url.host());
        if (!wp) {
            job->fail(QWebEngineUrlRequestJob::UrlNotFound);
            return;
        }
        QString rel = url.path();
        if (rel.isEmpty() || rel == "/")
            rel = "/" + wp->entry;
        if (rel.endsWith('/'))
            rel += "index.html";

        // Never serve anything outside the wallpaper directory.
        const QString path = QFileInfo(wp->dir + rel).canonicalFilePath();
        if (path.isEmpty() || !path.startsWith(wp->dir + "/") || !QFileInfo(path).isFile()) {
            job->fail(QWebEngineUrlRequestJob::UrlNotFound);
            return;
        }
        auto *file = new QFile(path, job);
        if (!file->open(QIODevice::ReadOnly)) {
            job->fail(QWebEngineUrlRequestJob::RequestFailed);
            return;
        }
        QByteArray mime = QMimeDatabase().mimeTypeForFile(path).name().toUtf8();
        if (path.endsWith(".js") || path.endsWith(".mjs"))
            mime = "text/javascript";
        else if (path.endsWith(".wasm"))
            mime = "application/wasm";
        job->reply(mime, file);
    }
};

// Injected into every page before its own scripts run.
const char *kBridge = R"JS(
(function () {
  if (window.undertow) return;
  const props = {};
  window.undertow = {
    stats: null,
    props,
    on(event, fn) {
      window.addEventListener('undertow:' + event, e => fn(e.detail));
      if (event === 'stats' && this.stats) fn(this.stats);
      if (event === 'prop') for (const k in props) fn({ name: k, value: props[k] });
    },
    _emit(event, detail) {
      if (event === 'stats') this.stats = detail;
      if (event === 'prop') {
        props[detail.name] = detail.value;
        // Lively Wallpaper compatibility
        try {
          if (typeof window.livelyPropertyListener === 'function')
            window.livelyPropertyListener(detail.name, detail.value);
        } catch (e) { console.error(e); }
      }
      window.dispatchEvent(new CustomEvent('undertow:' + event, { detail }));
    },
  };
})();
)JS";

// One full-screen web view pinned to an output's background layer.
// Qt may destroy and recreate the native window (it does so when the web
// content attaches), so the layer-shell setup is reapplied on every WinIdChange,
// which always arrives before the new surface is mapped.
class WallView : public QWebEngineView
{
public:
    WallView(QWebEngineProfile *profile, QScreen *screen)
        : QWebEngineView(profile, nullptr)
        , m_screen(screen)
    {
    }

    void configureLayer()
    {
        QWindow *window = windowHandle();
        if (!window || !m_screen)
            return;
        window->setScreen(m_screen);
        auto *layer = LayerShellQt::Window::get(window);
        layer->setScreen(m_screen);
        layer->setScope(QStringLiteral("undertow"));
        layer->setLayer(LayerShellQt::Window::LayerBackground);
        layer->setAnchors(LayerShellQt::Window::Anchors(LayerShellQt::Window::AnchorTop | LayerShellQt::Window::AnchorBottom
                                                        | LayerShellQt::Window::AnchorLeft | LayerShellQt::Window::AnchorRight));
        layer->setExclusiveZone(-1); // cover the whole output, ignore panels
        layer->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityNone);
    }

protected:
    bool event(QEvent *e) override
    {
        if (e->type() == QEvent::WinIdChange)
            configureLayer();
        return QWebEngineView::event(e);
    }

private:
    QPointer<QScreen> m_screen;
};

QString settingsKey(const QString &id, const QString &prop)
{
    return QStringLiteral("props/%1/%2").arg(QString::fromLatin1(id.toUtf8().toHex()), prop);
}

} // namespace

void Daemon::registerScheme()
{
    QWebEngineUrlScheme scheme(kScheme);
    scheme.setSyntax(QWebEngineUrlScheme::Syntax::Host);
    scheme.setFlags(QWebEngineUrlScheme::SecureScheme | QWebEngineUrlScheme::CorsEnabled
                    | QWebEngineUrlScheme::FetchApiAllowed);
    QWebEngineUrlScheme::registerScheme(scheme);
}

QString Daemon::socketName()
{
    return QStringLiteral("undertow-%1").arg(getuid());
}

Daemon::Daemon(QObject *parent)
    : QObject(parent)
{
    m_profile = new QWebEngineProfile(QStringLiteral("undertow"), this);
    m_profile->installUrlSchemeHandler(kScheme, new SchemeHandler(this));

    QWebEngineScript bridge;
    bridge.setName(QStringLiteral("undertow-bridge"));
    bridge.setSourceCode(QString::fromUtf8(kBridge));
    bridge.setInjectionPoint(QWebEngineScript::DocumentCreation);
    bridge.setWorldId(QWebEngineScript::MainWorld);
    m_profile->scripts()->insert(bridge);

    connect(&m_stats, &SysStats::sample, this, [this](const QJsonObject &s) { emitJs(QStringLiteral("stats"), s); });

    for (QScreen *s : QGuiApplication::screens())
        addScreen(s);
    connect(qGuiApp, &QGuiApplication::screenAdded, this, &Daemon::addScreen);
    connect(qGuiApp, &QGuiApplication::screenRemoved, this, &Daemon::removeScreen);
}

Daemon::~Daemon()
{
    // Pages must go before the profile they belong to.
    qDeleteAll(m_views);
    m_views.clear();
}

void Daemon::addScreen(QScreen *screen)
{
    if (m_views.contains(screen))
        return;
    auto *view = new WallView(m_profile, screen);
    view->setWindowTitle(QStringLiteral("undertow"));
    view->setWindowFlag(Qt::FramelessWindowHint);
    view->setAttribute(Qt::WA_ShowWithoutActivating);
    view->setFocusPolicy(Qt::NoFocus);
    view->setContextMenuPolicy(Qt::NoContextMenu);

    QWebEnginePage *page = view->page();
    page->setBackgroundColor(Qt::black);
    page->setAudioMuted(true);
    QWebEngineSettings *s = page->settings();
    s->setAttribute(QWebEngineSettings::ShowScrollBars, false);
    s->setAttribute(QWebEngineSettings::PlaybackRequiresUserGesture, false);
    s->setAttribute(QWebEngineSettings::WebGLEnabled, true);
    s->setAttribute(QWebEngineSettings::Accelerated2dCanvasEnabled, true);
    s->setAttribute(QWebEngineSettings::LocalContentCanAccessRemoteUrls, true);

    connect(view, &QWebEngineView::loadFinished, this, [this, view](bool ok) {
        if (ok)
            pushProps(view);
    });

    m_views.insert(screen, view);
    if (m_url.isValid())
        view->load(m_url);
    view->setGeometry(screen->geometry());
    view->winId(); // creates the native window -> WinIdChange -> configureLayer()
    view->configureLayer();
    view->show();
}

void Daemon::removeScreen(QScreen *screen)
{
    if (QWebEngineView *view = m_views.take(screen))
        view->deleteLater();
}

QString Daemon::apply(const QString &target)
{
    Wallpaper wp;
    QUrl url;
    if (target.startsWith("http://") || target.startsWith("https://")) {
        url = QUrl(target);
        wp.id = target;
        wp.title = target;
    } else if (auto found = Library::find(target)) {
        wp = *found;
        url.setScheme(QString::fromLatin1(kScheme));
        url.setHost(wp.id);
        url.setPath("/" + wp.entry);
    } else {
        return QStringLiteral("no wallpaper '%1' (see: undertow list)").arg(target);
    }
    if (!url.isValid())
        return QStringLiteral("bad URL '%1'").arg(target);

    m_current = wp;
    m_url = url;
    m_stats.setRunning(wp.stats);
    for (QWebEngineView *v : std::as_const(m_views))
        v->load(url);
    QSettings().setValue(QStringLiteral("current"), target);
    return {};
}

// Manifest defaults overlaid with whatever the user has set.
QJsonObject Daemon::propValues() const
{
    QJsonObject values;
    QSettings settings;
    for (auto it = m_current.props.begin(); it != m_current.props.end(); ++it) {
        const QJsonObject def = it.value().toObject();
        if (!def.contains("value"))
            continue; // labels, buttons
        const QVariant saved = settings.value(settingsKey(m_current.id, it.key()));
        values.insert(it.key(), saved.isValid() ? QJsonValue::fromVariant(saved) : def.value("value"));
    }
    return values;
}

void Daemon::pushProps(QWebEngineView *view)
{
    const QJsonObject values = propValues();
    for (auto it = values.begin(); it != values.end(); ++it)
        emitJs(QStringLiteral("prop"), QJsonObject{{"name", it.key()}, {"value", it.value()}}, view);
}

void Daemon::emitJs(const QString &event, const QJsonValue &detail, QWebEngineView *only)
{
    const QString args = QString::fromUtf8(QJsonDocument(QJsonArray{event, detail}).toJson(QJsonDocument::Compact));
    const QString js = QStringLiteral("window.undertow && window.undertow._emit(...%1);").arg(args);
    for (QWebEngineView *v : std::as_const(m_views))
        if (!only || v == only)
            v->page()->runJavaScript(js);
}

bool Daemon::listen()
{
    QLocalSocket probe;
    probe.connectToServer(socketName());
    if (probe.waitForConnected(300))
        return false; // another daemon is alive
    QLocalServer::removeServer(socketName());
    if (!m_server.listen(socketName()))
        return false;

    connect(&m_server, &QLocalServer::newConnection, this, [this] {
        while (QLocalSocket *sock = m_server.nextPendingConnection()) {
            connect(sock, &QLocalSocket::disconnected, sock, &QObject::deleteLater);
            connect(sock, &QLocalSocket::readyRead, this, [this, sock] {
                if (!sock->canReadLine())
                    return;
                const QString reply = handle(QString::fromUtf8(sock->readLine()).trimmed());
                sock->write(reply.toUtf8());
                sock->flush();
                sock->disconnectFromServer();
            });
        }
    });
    return true;
}

QString Daemon::handle(const QString &line)
{
    const QString cmd = line.section(' ', 0, 0);
    const QString rest = line.section(' ', 1).trimmed();

    if (cmd == "set") {
        const QString error = apply(rest);
        return error.isEmpty() ? QStringLiteral("now showing %1\n").arg(m_current.title) : "error: " + error + "\n";
    }
    if (cmd == "reload") {
        for (QWebEngineView *v : std::as_const(m_views))
            v->reload();
        return QStringLiteral("reloaded\n");
    }
    if (cmd == "status") {
        return QStringLiteral("wallpaper: %1\nurl: %2\nscreens: %3\nstats feed: %4\n")
            .arg(m_current.id.isEmpty() ? QStringLiteral("(none)") : m_current.id, m_url.toString())
            .arg(m_views.size())
            .arg(m_current.stats ? "on" : "off");
    }
    if (cmd == "props") {
        const QJsonObject values = propValues();
        if (values.isEmpty())
            return QStringLiteral("%1 has no properties\n").arg(m_current.id);
        QString text;
        for (auto it = values.begin(); it != values.end(); ++it) {
            const QJsonObject def = m_current.props.value(it.key()).toObject();
            QString range;
            if (def.contains("min") && def.contains("max"))
                range = QStringLiteral("  [%1..%2]").arg(def.value("min").toDouble()).arg(def.value("max").toDouble());
            text += QStringLiteral("%1 = %2%3   (%4)\n")
                        .arg(it.key(), it.value().toVariant().toString(), range,
                             def.value("text").toString(def.value("type").toString()));
        }
        return text;
    }
    if (cmd == "prop") {
        const QString key = rest.section(' ', 0, 0);
        const QString raw = rest.section(' ', 1).trimmed();
        if (!m_current.props.contains(key) || raw.isEmpty())
            return QStringLiteral("error: usage: undertow prop <name> <value>   (undertow props lists them)\n");
        // Keep the type of the default value.
        const QJsonValue def = m_current.props.value(key).toObject().value("value");
        QJsonValue value = raw;
        if (def.isDouble()) {
            bool ok = false;
            const double d = raw.toDouble(&ok);
            if (!ok)
                return QStringLiteral("error: %1 expects a number\n").arg(key);
            value = d;
        } else if (def.isBool()) {
            value = (raw == "1" || raw == "true" || raw == "on" || raw == "yes");
        }
        QSettings().setValue(settingsKey(m_current.id, key), value.toVariant());
        emitJs(QStringLiteral("prop"), QJsonObject{{"name", key}, {"value", value}});
        return QStringLiteral("%1 = %2\n").arg(key, value.toVariant().toString());
    }
    if (cmd == "quit") {
        QTimer::singleShot(0, qApp, &QCoreApplication::quit);
        return QStringLiteral("bye\n");
    }
    return QStringLiteral("error: unknown command '%1'\n").arg(cmd);
}
