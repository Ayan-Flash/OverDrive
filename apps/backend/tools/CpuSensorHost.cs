using System;
using System.Collections.Generic;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Diagnostics;
using System.IO;
using System.Security.Principal;
using System.ServiceProcess;
using System.Web.Script.Serialization;
using LibreHardwareMonitor.Hardware;

// Headless CPU sensor provider. Unlike LibreHardwareMonitor's tray UI, this
// explicitly updates the hardware tree and does not rely on a checkbox event
// to start its HTTP server.
internal static class CpuSensorHost
{
    private static readonly Computer Computer = new Computer { IsCpuEnabled = true };
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = Int32.MaxValue };

    /// Pre-serialized JSON body refreshed by the background updater thread.
    /// Requests read this reference (atomic for reference types) instead of
    /// walking and serializing the hardware tree per request, so a fetch costs
    /// far less than a millisecond.
    private static volatile byte[] CachedBody;
    private static volatile bool Running = true;
    private static readonly byte[] EmptyBody = Encoding.UTF8.GetBytes("{\"Text\":\"Computer\",\"Children\":[]}");
    private static readonly TcpListener Listener = new TcpListener(IPAddress.Loopback, 8085);
    private static string DiagnosticPath;
    private static bool ServiceMode;

    /// Refresh once per second to match the dashboard sampling cadence.
    private const int UpdateIntervalMs = 1000;

    private static void Main(string[] args)
    {
        if (args.Length == 1 && args[0] == "--service")
        {
            ServiceMode = true;
            ServiceBase.Run(new SensorService());
            return;
        }
        // Hold the actual parent process handle: PID reuse cannot keep an old
        // elevated provider alive after OverDrive has exited.
        if ((args.Length != 2 && args.Length != 4) || args[0] != "--parent") return;
        if (args.Length == 4 && args[2] == "--diagnostics") DiagnosticPath = args[3];
        int parentId;
        if (!Int32.TryParse(args[1], out parentId)) return;
        try
        {
            using (Process parent = Process.GetProcessById(parentId))
            {
                var handle = parent.Handle;
                if (parent.HasExited) return;
                var watcher = new Thread(delegate()
                {
                    try { parent.WaitForExit(); }
                    catch (InvalidOperationException) { }
                    finally { Running = false; Listener.Stop(); }
                }) { IsBackground = true };
                watcher.Start();
                Run();
            }
        }
        catch (Exception) { /* GUI subsystem: never display an exception dialog. */ }
        finally { Running = false; Listener.Stop(); Computer.Close(); }
    }

    private sealed class SensorService : ServiceBase
    {
        private Thread worker;
        public SensorService()
        {
            ServiceName = "OverDriveSensors";
            CanStop = true;
            CanShutdown = true;
            AutoLog = true;
        }

        protected override void OnStart(string[] args)
        {
            Running = true;
            CachedBody = null;
            worker = new Thread(delegate()
            {
                try { CpuSensorHost.Run(); }
                catch (Exception error)
                {
                    // Record startup failures without opening any desktop UI.
                    try { EventLog.WriteEntry(error.ToString(), EventLogEntryType.Error); }
                    finally { Environment.Exit(1); }
                }
                finally
                {
                    Running = false;
                    Listener.Stop();
                    Computer.Close();
                }
            }) { IsBackground = true };
            worker.Start();
        }

        protected override void OnStop()
        {
            Running = false;
            Listener.Stop();
            if (worker != null && !worker.Join(10000))
                throw new System.TimeoutException("Sensor worker did not stop in time.");
        }

        protected override void OnShutdown() { OnStop(); }
    }

    private static void Run()
    {
        Computer.Open();
        if (!Running) return;

        // Refresh the hardware tree and refresh the cached response on a
        // background thread; the request loop never touches the hardware tree.
        var updater = new Thread(UpdateLoop) { IsBackground = true };
        updater.Start();

        // Bind directly to loopback. HttpListener uses HTTP.sys URL ACLs on
        // Windows, which can reject standard per-user application installs.
        Listener.Start();
        while (Running)
        {
            try
            {
                using (TcpClient client = Listener.AcceptTcpClient())
                using (NetworkStream stream = client.GetStream())
                {
                    stream.ReadTimeout = 1500;
                    stream.WriteTimeout = 1500;
                    var request = new byte[2048];
                    int count = stream.Read(request, 0, request.Length);
                    string line = Encoding.ASCII.GetString(request, 0, count).Split('\n')[0];
                    bool dataRequest = line.StartsWith("GET /data.json ", StringComparison.OrdinalIgnoreCase);
                    byte[] body;
                    string status;
                    if (dataRequest)
                    {
                        // Briefly wait for the first background update so the
                        // app's startup probe sees a valid sensor tree.
                        for (int waited = 0; CachedBody == null && waited < 2000; waited += 25) Thread.Sleep(25);
                        body = CachedBody ?? Encoding.UTF8.GetBytes("{ \"Text\": \"Computer\", \"Children\": [] }");
                        status = "200 OK";
                    }
                    else
                    {
                        body = Encoding.UTF8.GetBytes("Not found");
                        status = "404 Not Found";
                    }
                    byte[] header = Encoding.ASCII.GetBytes("HTTP/1.1 " + status + "\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: " + body.Length.ToString(CultureInfo.InvariantCulture) + "\r\nConnection: close\r\n\r\n");
                    stream.Write(header, 0, header.Length);
                    stream.Write(body, 0, body.Length);
                }
            }
            catch (SocketException) { break; }
            catch (Exception) { Thread.Sleep(100); }
        }
        Running = false;
        updater.Join(3000);
    }

    private static void UpdateLoop()
    {
        while (Running)
        {
            try
            {
                Computer.Accept(new UpdateVisitor());
                CachedBody = Encoding.UTF8.GetBytes(Json.Serialize(BuildRoot()));
                if (DiagnosticPath != null)
                {
                    string path = DiagnosticPath;
                    DiagnosticPath = null;
                    try { File.WriteAllText(path, Computer.GetReport()); } catch (IOException) { }
                }
            }
            catch (Exception)
            {
                // Failed reads must not masquerade as a frozen live sensor.
                CachedBody = EmptyBody;
            }
            Thread.Sleep(UpdateIntervalMs);
        }
    }

    private static Dictionary<string, object> BuildRoot()
    {
        var children = new List<object>();
        foreach (IHardware hardware in Computer.Hardware) children.Add(BuildHardware(hardware));
        bool elevated = new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator);
        return new Dictionary<string, object> {
            { "Text", "Computer" }, { "Children", children }, { "IsElevated", elevated },
            { "DriverInstalled", LibreHardwareMonitor.PawnIo.PawnIo.IsInstalled },
            { "Provider", "OverDriveSensors" }, { "ServiceMode", ServiceMode }
        };
    }

    private static Dictionary<string, object> BuildHardware(IHardware hardware)
    {
        var children = new List<object>();
        foreach (ISensor sensor in hardware.Sensors)
        {
            if (sensor.SensorType != SensorType.Temperature && sensor.SensorType != SensorType.Clock) continue;
            var item = new Dictionary<string, object>
            {
                { "Text", sensor.Name }, { "SensorId", sensor.Identifier.ToString() },
                { "SensorType", sensor.SensorType.ToString() }
            };
            if (sensor.Value.HasValue)
            {
                double value = Math.Round(sensor.Value.Value, 2);
                item["RawValue"] = value;
                item["Value"] = value.ToString("0.##", CultureInfo.InvariantCulture);
            }
            children.Add(item);
        }
        foreach (IHardware subHardware in hardware.SubHardware) children.Add(BuildHardware(subHardware));
        return new Dictionary<string, object> { { "Text", hardware.Name }, { "Children", children } };
    }

    private sealed class UpdateVisitor : IVisitor
    {
        public void VisitComputer(IComputer computer) { computer.Traverse(this); }
        public void VisitHardware(IHardware hardware) { hardware.Update(); foreach (IHardware child in hardware.SubHardware) child.Accept(this); }
        public void VisitSensor(ISensor sensor) { }
        public void VisitParameter(IParameter parameter) { }
    }
}
