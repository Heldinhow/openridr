Console.WriteLine($"PROFILE_ENV={Environment.GetEnvironmentVariable("PROFILE_ENV")}");
Console.WriteLine($"EXPANDED={Environment.GetEnvironmentVariable("EXPANDED")}");
Console.WriteLine($"ASPNETCORE_URLS={Environment.GetEnvironmentVariable("ASPNETCORE_URLS")}");
Console.WriteLine($"ARGS={string.Join("|", args)}");
Console.WriteLine($"CWD={Environment.CurrentDirectory}");
