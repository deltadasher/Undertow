#pragma once
#include <QJsonObject>
#include <QList>
#include <QString>
#include <optional>

// A wallpaper is a directory containing a web page.
// Recognised layouts, in order:
//   undertow.json                       { "name", "entry", "feeds": ["stats"], "props": {...} }
//   LivelyInfo.json (+ LivelyProperties.json)   Lively Wallpaper packages
//   index.html                          any plain web project
struct Wallpaper {
    QString id;        // directory name, lowercase; also the host in undertow://<id>/
    QString title;
    QString dir;       // canonical path
    QString entry;     // path of the page relative to dir
    bool builtin = false;
    bool stats = false;   // wants the system stats feed
    QJsonObject props;    // { key: { "type", "text", "value", ... } }  (Lively-compatible)
};

namespace Library {
QString userDir();
QList<Wallpaper> all();
std::optional<Wallpaper> find(const QString &id);

// CLI actions; return a process exit code and print to stdout/stderr.
int add(const QString &spec);
int update(const QString &id);
int remove(const QString &id);
}
