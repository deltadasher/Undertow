#pragma once
#include <QHash>
#include <QJsonObject>
#include <QList>
#include <QObject>
#include <QTimer>

// Samples /proc once a second and emits a JSON snapshot for wallpapers
// that ask for the "stats" feed.
class SysStats : public QObject
{
    Q_OBJECT
public:
    explicit SysStats(QObject *parent = nullptr);
    void setRunning(bool on);

signals:
    void sample(const QJsonObject &stats);

private:
    struct Cpu {
        quint64 total = 0, idle = 0;
    };
    void tick();

    QTimer m_timer;
    QList<Cpu> m_prevCpu;                 // [0] is the aggregate line
    QHash<int, quint64> m_prevProcTicks;  // pid -> utime+stime
    quint64 m_prevRx = 0, m_prevTx = 0;
    bool m_haveNet = false;
    QString m_tempPath;
};
