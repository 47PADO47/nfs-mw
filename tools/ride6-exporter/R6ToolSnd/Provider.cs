using CUE4Parse.Compression;
using CUE4Parse.Encryption.Aes;
using CUE4Parse.FileProvider;
using CUE4Parse.MappingsProvider.Usmap;
using CUE4Parse.UE4.Objects.Core.Misc;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse.UE4.Versions;

namespace R6ToolSnd;

/// <summary>Opens the RIDE 6 install: the IoStore containers need no key, the .pak containers (the Wwise banks) need the
/// AES key from <c>R6_AES</c>, and packages need the .usmap (<c>R6_USMAP</c>).</summary>
public static class Provider
{
    public const string DefaultPaks = @"D:\RIDE 6\ride6\Content\Paks";
    public const string DefaultUsmapDir = @"C:\Dumper-7\5.6.1-1697004+++ride6+rel-submission-ride6\Mappings";

    public static DefaultFileProvider Open(string game)
    {
        var paks = Environment.GetEnvironmentVariable("R6_PAKS") ?? DefaultPaks;
        var usmap = Environment.GetEnvironmentVariable("R6_USMAP");
        if (string.IsNullOrEmpty(usmap))
            usmap = Directory.GetFiles(DefaultUsmapDir, "*.usmap").OrderBy(f => f).First();
        var key = Environment.GetEnvironmentVariable("R6_AES");
        if (string.IsNullOrEmpty(key)) throw new InvalidOperationException("R6_AES (the pak key, 64 hex digits) is not set");
        if (!key.StartsWith("0x", StringComparison.OrdinalIgnoreCase)) key = "0x" + key;
        // The native decompressors are downloaded once, next to the executable (not into the working directory).
        ZlibHelper.Initialize(Path.Combine(AppContext.BaseDirectory, ZlibHelper.DLL_NAME));
        OodleHelper.Initialize(Path.Combine(AppContext.BaseDirectory, OodleHelper.OodleFileName));
        var engine = Enum.Parse<EGame>(game);
        var version = new VersionContainer(engine, ETexturePlatform.DesktopMobile);
        var provider = new DefaultFileProvider(paks, SearchOption.TopDirectoryOnly, version, StringComparer.OrdinalIgnoreCase)
        {
            MappingsContainer = new FileUsmapTypeMappingsProvider(usmap),
        };
        provider.Initialize();
        provider.Mount();
        var mounted = provider.SubmitKey(new FGuid(), new FAesKey(key));
        provider.PostMount();
        Console.Error.WriteLine($"mounted {provider.MountedVfs.Count} containers ({mounted} with the key), {provider.Files.Count} files");
        return provider;
    }
}
