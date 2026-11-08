namespace R6ToolSnd;

/// <summary>Console entry: <c>R6ToolSnd &lt;game&gt; raw &lt;regex&gt; &lt;folder&gt;</c> or
/// <c>R6ToolSnd &lt;game&gt; audiomap &lt;folder&gt;</c> (<c>game</c> is the CUE4Parse engine name, <c>GAME_UE5_6</c>). The
/// environment: <c>R6_AES</c> (pak key), <c>R6_USMAP</c> (mappings), <c>R6_PAKS</c> (the <c>Paks</c> folder).</summary>
public static class Program
{
    private const string Usage = """
        R6ToolSnd <game> raw <regex> <folder>   save every file of the install whose path matches the regex
        R6ToolSnd <game> audiomap <folder>      one <chassis>__<Stock|Custom>.json per row of the chassis audio table
        (env R6_AES = pak key, R6_USMAP = .usmap, R6_PAKS = Paks folder)
        """;

    public static int Main(string[] args)
    {
        if (args.Length < 3)
        {
            Console.Error.WriteLine(Usage);
            return 2;
        }
        var provider = Provider.Open(args[0]);
        if (args[1] == "raw" && args.Length == 4) return Raw.Run(provider, args[2], args[3]);
        if (args[1] == "audiomap" && args.Length == 3) return AudioMap.Run(provider, args[2]);
        Console.Error.WriteLine(Usage);
        return 2;
    }
}
