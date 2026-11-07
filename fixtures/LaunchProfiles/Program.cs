using System.Net;
using System.Text;

Console.WriteLine($"PROFILE_ENV={Environment.GetEnvironmentVariable("PROFILE_ENV")}");
Console.WriteLine($"EXPANDED={Environment.GetEnvironmentVariable("EXPANDED")}");
Console.WriteLine($"ASPNETCORE_URLS={Environment.GetEnvironmentVariable("ASPNETCORE_URLS")}");
Console.WriteLine($"ARGS={string.Join("|", args)}");
Console.WriteLine($"CWD={Environment.CurrentDirectory}");

// The browser profiles need a real listener: the host opens the browser only when this port
// accepts connections. Every other profile prints and exits, as before.
if (args.Contains("--listen"))
{
    var urls = (Environment.GetEnvironmentVariable("ASPNETCORE_URLS") ?? "http://localhost:5090")
        .Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
    using var listener = new HttpListener();
    foreach (var url in urls)
    {
        listener.Prefixes.Add(url.TrimEnd('/') + "/");
    }
    listener.Start();
    Console.WriteLine($"LISTENING={string.Join(";", listener.Prefixes)}");
    Console.Out.Flush();
    var page = Encoding.UTF8.GetBytes(
        "<!doctype html><meta charset=\"utf-8\"><title>Launch profile</title>"
            + "<h1>OpenRidr launch profile</h1><p>This page was opened by the launch profile.</p>"
    );
    while (listener.IsListening)
    {
        var context = await listener.GetContextAsync();
        context.Response.ContentType = "text/html; charset=utf-8";
        context.Response.ContentLength64 = page.Length;
        await context.Response.OutputStream.WriteAsync(page);
        context.Response.Close();
    }
}
