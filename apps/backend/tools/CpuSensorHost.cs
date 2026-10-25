using System;
using System.Collections.Generic;
using System.Globalization;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;
using LibreHardwareMonitor.Hardware;

// Headless CPU sensor provider. Unlike LibreHardwareMonitor's tray UI, this
// explicitly updates the hardware tree and does not rely on a checkbox event
// to start its HTTP server.
internal static class CpuSensorHost
{
    private static readonly Computer Computer = new Computer { IsCpuEnabled = true };
    private static readonly JavaScriptSerializer Json = new JavaScriptSerializer { MaxJsonLength = Int32.MaxValue };

    private static void Main()
    {
        Computer.Open();
        // Bind directly to loopback. HttpListener uses HTTP.sys URL ACLs on
        // Windows, which can reject standard per-user application installs.
        var listener = new TcpListener(IPAddress.Loopback, 8085);
        listener.Start();
        while (true)
        {
            try
            {
                using (TcpClient client = listener.AcceptTcpClient())
                using (NetworkStream stream = client.GetStream())
                {
                    var request = new byte[2048];
                    int count = stream.Read(request, 0, request.Length);
                    string line = Encoding.ASCII.GetString(request, 0, count).Split('\n')[0];
                    bool dataRequest = line.StartsWith("GET /data.json ", StringComparison.OrdinalIgnoreCase);
                    byte[] body;
                    string status;
                    if (dataRequest)
                    {
                        Computer.Accept(new UpdateVisitor());
                        body = Encoding.UTF8.GetBytes(Json.Serialize(BuildRoot()));
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
        Computer.Close();
    }

    private static Dictionary<string, object> BuildRoot()
    {
        var children = new List<object>();
        foreach (IHardware hardware in Computer.Hardware) children.Add(BuildHardware(hardware));
        return new Dictionary<string, object> { { "Text", "Computer" }, { "Children", children } };
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
