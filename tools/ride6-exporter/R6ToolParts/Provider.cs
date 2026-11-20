using CUE4Parse.Compression;
using CUE4Parse.FileProvider;
using CUE4Parse.FileProvider.Objects;
using CUE4Parse.MappingsProvider.Usmap;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse.UE4.Versions;

namespace R6ToolParts;

/// <summary>Opens the RIDE 6 install: IoStore containers need no key, packages need the .usmap.</summary>
public static class Provider
{
    public const string DefaultPaks = @"D:\RIDE 6\ride6\Content\Paks";
    public const string DefaultUsmapDir = @"C:\Dumper-7\5.6.1-1697004+++ride6+rel-submission-ride6\Mappings";

    public static DefaultFileProvider Open()
    {
        var paks = Environment.GetEnvironmentVariable("R6_PAKS") ?? DefaultPaks;
        var usmap = Environment.GetEnvironmentVariable("R6_USMAP");
        if (string.IsNullOrEmpty(usmap))
            usmap = Directory.GetFiles(DefaultUsmapDir, "*.usmap").OrderBy(f => f).First();
        // The native decompressors are downloaded once, next to the executable (not into the working directory).
        ZlibHelper.Initialize(Path.Combine(AppContext.BaseDirectory, ZlibHelper.DLL_NAME));
        OodleHelper.Initialize(Path.Combine(AppContext.BaseDirectory, OodleHelper.OodleFileName));
        var version = new VersionContainer(EGame.GAME_UE5_6, ETexturePlatform.DesktopMobile);
        var provider = new DefaultFileProvider(paks, SearchOption.TopDirectoryOnly, version, StringComparer.OrdinalIgnoreCase)
        {
            MappingsContainer = new FileUsmapTypeMappingsProvider(usmap),
        };
        provider.Initialize();
        provider.Mount();
        provider.PostMount();
        return provider;
    }

    /// <summary>The game file of a package path: an Unreal object path (<c>/Game/x/y.y</c>, <c>/Plugin/x/y</c>) or a
    /// file-system style path (<c>ride6/Content/x/y</c>), with or without extension.</summary>
    public static GameFile? Find(DefaultFileProvider provider, string path)
    {
        path = path.Trim().Replace('\\', '/');
        var dot = path.LastIndexOf('.');
        var slash = path.LastIndexOf('/');
        if (dot > slash && !path.EndsWith(".uasset") && !path.EndsWith(".umap")) path = path[..dot];
        foreach (var candidate in Candidates(path))
        {
            foreach (var ext in new[] { "", ".uasset", ".umap" })
                if (provider.Files.TryGetValue(candidate + ext, out var file) && file.IsUePackage) return file;
        }
        return null;
    }

    private static IEnumerable<string> Candidates(string path)
    {
        yield return path;
        var trimmed = path.TrimStart('/');
        yield return trimmed;
        var first = trimmed.IndexOf('/');
        if (first < 0) yield break;
        var mount = trimmed[..first];
        var rest = trimmed[(first + 1)..];
        if (mount.Equals("Game", StringComparison.OrdinalIgnoreCase))
            yield return "ride6/Content/" + rest;
        else
            yield return $"ride6/Plugins/{mount}/Content/{rest}";
    }
}
