#include "library.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QProcess>
#include <QRegularExpression>
#include <QSet>
#include <QStandardPaths>
#include <QTextStream>

namespace {

QTextStream &out()
{
    static QTextStream s(stdout);
    return s;
}
QTextStream &err()
{
    static QTextStream s(stderr);
    return s;
}

QJsonObject readJson(const QString &path)
{
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly))
        return {};
    return QJsonDocument::fromJson(f.readAll()).object();
}

// Directories searched for wallpapers. User dir first so it can shadow presets.
QList<QPair<QString, bool>> searchDirs()
{
    QList<QPair<QString, bool>> dirs;
    dirs.append({Library::userDir(), false});
    const QString bin = QCoreApplication::applicationDirPath();
    // Running from a build dir inside the source tree, or installed under a prefix.
    for (const QString &rel : {QStringLiteral("/../presets"), QStringLiteral("/../share/undertow/wallpapers")}) {
        const QString p = QDir(bin + rel).canonicalPath();
        if (!p.isEmpty())
            dirs.append({p, true});
    }
    return dirs;
}

std::optional<Wallpaper> load(const QString &path, bool builtin)
{
    const QFileInfo info(path);
    const QString dir = info.canonicalFilePath();
    if (dir.isEmpty() || !QFileInfo(dir).isDir())
        return std::nullopt;

    Wallpaper w;
    w.id = info.fileName().toLower();
    w.title = info.fileName();
    w.dir = dir;
    w.builtin = builtin;
    w.entry = QStringLiteral("index.html");

    const QJsonObject own = readJson(dir + "/undertow.json");
    const QJsonObject lively = readJson(dir + "/LivelyInfo.json");
    if (!own.isEmpty()) {
        w.title = own.value("name").toString(w.title);
        w.entry = own.value("entry").toString(w.entry);
        w.props = own.value("props").toObject();
        for (const QJsonValue &f : own.value("feeds").toArray())
            if (f.toString() == QLatin1String("stats"))
                w.stats = true;
    } else if (!lively.isEmpty()) {
        w.title = lively.value("Title").toString(w.title);
        w.entry = lively.value("FileName").toString(w.entry);
    }
    if (w.props.isEmpty())
        w.props = readJson(dir + "/LivelyProperties.json");

    if (!QFileInfo::exists(dir + "/" + w.entry))
        return std::nullopt;
    return w;
}

int git(const QStringList &args, const QString &cwd = {})
{
    QProcess p;
    p.setProcessChannelMode(QProcess::ForwardedChannels);
    if (!cwd.isEmpty())
        p.setWorkingDirectory(cwd);
    p.start(QStringLiteral("git"), args);
    if (!p.waitForFinished(-1) || p.exitStatus() != QProcess::NormalExit)
        return 1;
    return p.exitCode();
}

} // namespace

QString Library::userDir()
{
    return QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation) + "/undertow/wallpapers";
}

QList<Wallpaper> Library::all()
{
    QList<Wallpaper> list;
    QSet<QString> seen;
    for (const auto &[dir, builtin] : searchDirs()) {
        const auto entries = QDir(dir).entryInfoList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);
        for (const QFileInfo &e : entries) {
            auto w = load(e.filePath(), builtin);
            if (w && !seen.contains(w->id)) {
                seen.insert(w->id);
                list.append(*w);
            }
        }
    }
    return list;
}

std::optional<Wallpaper> Library::find(const QString &id)
{
    const QString want = id.toLower();
    for (const Wallpaper &w : all())
        if (w.id == want)
            return w;
    return std::nullopt;
}

// Accepted specs:
//   owner/repo            owner/repo/sub/dir          owner/repo@branch
//   https://github.com/owner/repo[/tree/<ref>/<sub/dir>]
//   any other git URL
//   /path/to/local/dir    (symlinked into the library, handy while developing)
int Library::add(const QString &specIn)
{
    QDir().mkpath(userDir());
    QString spec = specIn;

    const QFileInfo local(spec);
    if (local.isDir()) {
        const QString id = local.canonicalFilePath().section('/', -1).toLower();
        const QString dest = userDir() + "/" + id;
        if (QFileInfo::exists(dest)) {
            err() << "undertow: '" << id << "' already exists\n";
            return 1;
        }
        if (!QFile::link(local.canonicalFilePath(), dest)) {
            err() << "undertow: could not link " << dest << "\n";
            return 1;
        }
        out() << "linked " << id << " -> " << local.canonicalFilePath() << "\n";
        return 0;
    }

    QString url, ref, sub;
    static const QRegularExpression ghUrl(
        QStringLiteral(R"(^https?://github\.com/([^/]+)/([^/]+?)(?:\.git)?(?:/tree/([^/]+)(?:/(.+?))?)?/?$)"));
    static const QRegularExpression shortSpec(QStringLiteral(R"(^([\w.-]+)/([\w.-]+)((?:/[^@]+)?)(?:@(.+))?$)"));

    QString repo;
    if (auto m = ghUrl.match(spec); m.hasMatch()) {
        repo = m.captured(2);
        url = QStringLiteral("https://github.com/%1/%2.git").arg(m.captured(1), repo);
        ref = m.captured(3);
        sub = m.captured(4);
    } else if (auto s = shortSpec.match(spec); s.hasMatch() && !spec.contains("://")) {
        repo = s.captured(2);
        url = QStringLiteral("https://github.com/%1/%2.git").arg(s.captured(1), repo);
        sub = s.captured(3).mid(1);
        ref = s.captured(4);
    } else if (spec.contains("://") || spec.startsWith("git@")) {
        url = spec;
        repo = spec.section('/', -1);
        if (repo.endsWith(".git"))
            repo.chop(4);
    } else {
        err() << "undertow: don't know how to fetch '" << spec << "'\n"
              << "  try: owner/repo, owner/repo/sub/dir, a GitHub URL, or a local directory\n";
        return 1;
    }
    while (sub.endsWith('/'))
        sub.chop(1);

    QString id = repo.toLower();
    if (!sub.isEmpty())
        id += "-" + sub.section('/', -1).toLower();
    id.replace(QRegularExpression(QStringLiteral("[^a-z0-9.-]")), QStringLiteral("-"));

    const QString dest = userDir() + "/" + id;
    if (QFileInfo::exists(dest)) {
        err() << "undertow: '" << id << "' is already installed (undertow update " << id << ")\n";
        return 1;
    }

    QStringList args{"clone", "--depth", "1"};
    if (!ref.isEmpty())
        args << "--branch" << ref;
    args << "--" << url << dest;
    if (git(args) != 0) {
        err() << "undertow: git clone failed\n";
        return 1;
    }

    // Point at a page in a subdirectory without touching tracked files.
    if (!sub.isEmpty() && !QFileInfo::exists(dest + "/undertow.json")) {
        QFile f(dest + "/undertow.json");
        if (f.open(QIODevice::WriteOnly)) {
            QString entry = sub;
            if (!entry.endsWith(".html") && !entry.endsWith(".htm"))
                entry += "/index.html";
            f.write(QJsonDocument(QJsonObject{{"name", id}, {"entry", entry}}).toJson());
        }
    }

    if (!find(id)) {
        err() << "undertow: cloned to " << dest << " but found no page to show.\n"
              << "  If the page lives in a subdirectory: undertow remove " << id << " && undertow add "
              << specIn << "/<subdir>\n"
              << "  If the project needs a build step (npm etc.), build it there and point\n"
              << "  \"entry\" in " << dest << "/undertow.json at the built index.html.\n";
        return 2;
    }
    out() << "installed " << id << "\n  undertow set " << id << "\n";
    return 0;
}

int Library::update(const QString &id)
{
    QList<Wallpaper> targets;
    if (id.isEmpty()) {
        targets = all();
    } else if (auto w = find(id)) {
        targets.append(*w);
    } else {
        err() << "undertow: no wallpaper '" << id << "'\n";
        return 1;
    }
    int rc = 0;
    for (const Wallpaper &w : targets) {
        if (w.builtin || !QFileInfo::exists(w.dir + "/.git"))
            continue;
        out() << "== " << w.id << "\n";
        out().flush();
        rc |= git({"pull", "--ff-only"}, w.dir);
    }
    return rc;
}

int Library::remove(const QString &id)
{
    const QString path = userDir() + "/" + id.toLower();
    const QFileInfo info(path);
    if (id.isEmpty() || id.contains('/') || id.startsWith('.') || (!info.exists() && !info.isSymLink())) {
        err() << "undertow: '" << id << "' is not an installed wallpaper (built-in presets can't be removed)\n";
        return 1;
    }
    const bool ok = info.isSymLink() ? QFile::remove(path) : QDir(path).removeRecursively();
    if (!ok) {
        err() << "undertow: could not remove " << path << "\n";
        return 1;
    }
    out() << "removed " << id.toLower() << "\n";
    return 0;
}
