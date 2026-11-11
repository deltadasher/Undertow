#include "sysstats.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QSysInfo>
#include <algorithm>
#include <unistd.h>

namespace {

QByteArray slurp(const QString &path)
{
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly))
        return {};
    return f.readAll();
}

QString findCpuTemp()
{
    static const QList<QByteArray> wanted{"coretemp", "k10temp", "zenpower", "cpu_thermal"};
    const QDir hwmon(QStringLiteral("/sys/class/hwmon"));
    for (const QString &d : hwmon.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        const QString base = hwmon.filePath(d);
        if (wanted.contains(slurp(base + "/name").trimmed()) && QFile::exists(base + "/temp1_input"))
            return base + "/temp1_input";
    }
    return {};
}

} // namespace

SysStats::SysStats(QObject *parent)
    : QObject(parent)
    , m_tempPath(findCpuTemp())
{
    m_timer.setInterval(1000);
    connect(&m_timer, &QTimer::timeout, this, &SysStats::tick);
}

void SysStats::setRunning(bool on)
{
    if (on == m_timer.isActive())
        return;
    if (on) {
        m_prevCpu.clear();
        m_prevProcTicks.clear();
        m_haveNet = false;
        tick(); // prime the deltas
        m_timer.start();
    } else {
        m_timer.stop();
    }
}

void SysStats::tick()
{
    QJsonObject o;

    // --- CPU -------------------------------------------------------------
    QList<Cpu> now;
    for (const QByteArray &line : slurp(QStringLiteral("/proc/stat")).split('\n')) {
        if (!line.startsWith("cpu"))
            break;
        const QList<QByteArray> f = line.simplified().split(' ');
        if (f.size() < 9)
            continue;
        Cpu c;
        for (int i = 1; i <= 8; ++i)
            c.total += f[i].toULongLong();
        c.idle = f[4].toULongLong() + f[5].toULongLong();
        now.append(c);
    }
    const bool haveCpu = m_prevCpu.size() == now.size() && !now.isEmpty();
    QJsonArray cores;
    double totalUse = 0;
    quint64 jiffiesPerCore = 0;
    if (haveCpu) {
        for (int i = 0; i < now.size(); ++i) {
            const quint64 dt = now[i].total - m_prevCpu[i].total;
            const quint64 di = now[i].idle - m_prevCpu[i].idle;
            const double use = dt ? 1.0 - double(di) / double(dt) : 0.0;
            if (i == 0) {
                totalUse = use;
                jiffiesPerCore = dt / std::max<qsizetype>(1, now.size() - 1);
            } else {
                cores.append(use);
            }
        }
    }
    m_prevCpu = now;
    o["cpu"] = QJsonObject{{"total", totalUse}, {"cores", cores}};

    // --- Memory (kB) -----------------------------------------------------
    QHash<QByteArray, double> mem;
    for (const QByteArray &line : slurp(QStringLiteral("/proc/meminfo")).split('\n')) {
        const int colon = line.indexOf(':');
        if (colon > 0)
            mem.insert(line.left(colon), line.mid(colon + 1).trimmed().split(' ').value(0).toDouble());
    }
    o["mem"] = QJsonObject{
        {"total", mem.value("MemTotal")},
        {"used", mem.value("MemTotal") - mem.value("MemAvailable")},
        {"swapTotal", mem.value("SwapTotal")},
        {"swapUsed", mem.value("SwapTotal") - mem.value("SwapFree")},
    };

    // --- Network (bytes/s, all interfaces except lo) -----------------------
    quint64 rx = 0, tx = 0;
    const QList<QByteArray> netLines = slurp(QStringLiteral("/proc/net/dev")).split('\n');
    for (int i = 2; i < netLines.size(); ++i) {
        const int colon = netLines[i].indexOf(':');
        if (colon < 0 || netLines[i].left(colon).trimmed() == "lo")
            continue;
        const QList<QByteArray> f = netLines[i].mid(colon + 1).simplified().split(' ');
        if (f.size() >= 9) {
            rx += f[0].toULongLong();
            tx += f[8].toULongLong();
        }
    }
    o["net"] = QJsonObject{
        {"rx", m_haveNet && rx >= m_prevRx ? double(rx - m_prevRx) : 0.0},
        {"tx", m_haveNet && tx >= m_prevTx ? double(tx - m_prevTx) : 0.0},
    };
    m_prevRx = rx;
    m_prevTx = tx;
    m_haveNet = true;

    // --- Load, uptime, temperature, identity -------------------------------
    const QList<QByteArray> load = slurp(QStringLiteral("/proc/loadavg")).split(' ');
    o["load"] = QJsonArray{load.value(0).toDouble(), load.value(1).toDouble(), load.value(2).toDouble()};
    o["uptime"] = slurp(QStringLiteral("/proc/uptime")).split(' ').value(0).toDouble();
    if (!m_tempPath.isEmpty())
        o["temp"] = slurp(m_tempPath).trimmed().toDouble() / 1000.0;
    o["host"] = QSysInfo::machineHostName();
    o["kernel"] = QSysInfo::kernelVersion();

    // --- Top processes -----------------------------------------------------
    struct Proc {
        int pid;
        QString name;
        double cpu;
        double rssKb;
    };
    QList<Proc> procs;
    QHash<int, quint64> ticksNow;
    const double pageKb = sysconf(_SC_PAGESIZE) / 1024.0;
    const QStringList pids = QDir(QStringLiteral("/proc")).entryList(QDir::Dirs | QDir::NoDotAndDotDot);
    for (const QString &name : pids) {
        bool isPid = false;
        const int pid = name.toInt(&isPid);
        if (!isPid)
            continue;
        const QByteArray stat = slurp("/proc/" + name + "/stat");
        // "pid (comm) state ppid ..." - comm may itself contain spaces and parens.
        const int open = stat.indexOf('(');
        const int close = stat.lastIndexOf(')');
        if (open < 0 || close < open)
            continue;
        const QList<QByteArray> f = stat.mid(close + 2).split(' ');
        if (f.size() < 22)
            continue;
        const quint64 ticks = f[11].toULongLong() + f[12].toULongLong();
        ticksNow.insert(pid, ticks);
        const auto prev = m_prevProcTicks.constFind(pid);
        double cpu = 0;
        if (prev != m_prevProcTicks.constEnd() && jiffiesPerCore && ticks >= *prev)
            cpu = 100.0 * double(ticks - *prev) / double(jiffiesPerCore);
        procs.append({pid, QString::fromUtf8(stat.mid(open + 1, close - open - 1)), cpu, f[21].toDouble() * pageKb});
    }
    m_prevProcTicks = ticksNow;

    const int top = std::min<qsizetype>(10, procs.size());
    std::partial_sort(procs.begin(), procs.begin() + top, procs.end(), [](const Proc &a, const Proc &b) {
        return a.cpu != b.cpu ? a.cpu > b.cpu : a.rssKb > b.rssKb;
    });
    QJsonArray procArr;
    for (int i = 0; i < top; ++i)
        procArr.append(QJsonObject{{"pid", procs[i].pid}, {"name", procs[i].name}, {"cpu", procs[i].cpu}, {"mem", procs[i].rssKb}});
    o["procs"] = procArr;
    o["nprocs"] = int(procs.size());

    if (haveCpu)
        emit sample(o);
}
