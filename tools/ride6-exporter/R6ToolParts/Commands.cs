using System.Text.RegularExpressions;
using CUE4Parse.FileProvider;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse_Conversion;
using CUE4Parse_Conversion.Options;
using CUE4Parse_Conversion.Textures;
using Newtonsoft.Json;

namespace R6ToolParts;

public static class Commands
{
    public const string Usage = """
        R6ToolParts <mode> args   (env R6_PAKS, R6_USMAP)
          list <regex> [outfile]                file paths matching the regex (case insensitive)
          jsonmany <listfile> <outdir>          the exports of each listed package as <outdir>/<package path>.json
          tex <listfile> <outdir> <max>         each texture as <stem>.png (+ .txt), largest mip not above <max> px
          export "<pkg;pkg>" <outdir> Gltf2 All every level of detail of each mesh as .glb
          raw <listfile> <outdir>               the bytes of each listed file (any extension, path as listed)
          gmesh <listfile> <outdir>             each listed skeletal mesh as <name>.r6rm (rider gear, docs/formats/ride6-rider-gear.md)
          gtex <listfile> <outdir> <max>        each listed texture as <name>.r6tx: its block-compressed mips from the largest not above <max> px
          bp <listfile> <outdir>                each listed Blueprint class as pseudo-C++ (<class>.cpp)
          skeleton <package> <outfile>          the skeleton of an animation sequence as skeleton.json (rider, docs/formats/ride6-rider.md)
          anims <listfile> <outfile.r6ra>       each listed animation sequence (Unreal's own codecs) sampled at every frame, as R6RA
          acl <listfile> <outdir>               each listed ACL animation sequence as <name>.acl and <name>.meta.json
          serve                                 requests as tab separated lines on stdin
        """;

    public static bool Run(DefaultFileProvider provider, string[] args, TextWriter output)
    {
        switch (args[0])
        {
            case "list": return List(provider, args, output);
            case "jsonmany": return JsonMany(provider, args[1], args[2], output);
            case "tex": return Textures(provider, args[1], args[2], int.Parse(args[3]), output);
            case "export": return Export(provider, args[1], args[2], args.Length > 3 ? args[3] : "Gltf2", output);
            case "raw": return Raw(provider, args[1], args[2], output);
            case "gmesh": return Gear.Meshes(provider, args[1], args[2], output);
            case "gtex": return Gear.Textures(provider, args[1], args[2], int.Parse(args[3]), output);
            case "bp": return Bp.Classes(provider, args[1], args[2], output);
            case "skeleton": return Anim.Skeleton(provider, args[1], args[2], output);
            case "anims": return Anim.Anims(provider, args[1], args[2], output);
            case "acl": return Anim.Acl(provider, args[1], args[2], output);
            case "ping":
                output.WriteLine("pong");
                return true;
            default:
                output.WriteLine(Usage);
                return false;
        }
    }

    private static IEnumerable<string> Lines(string listFile) =>
        File.ReadAllLines(listFile).Select(l => l.Trim()).Where(l => l.Length > 0 && !l.StartsWith('#'));

    private static bool List(DefaultFileProvider provider, string[] args, TextWriter output)
    {
        var regex = new Regex(args[1], RegexOptions.IgnoreCase | RegexOptions.Compiled);
        var names = provider.Files.Keys.Where(k => regex.IsMatch(k)).OrderBy(k => k, StringComparer.Ordinal).ToList();
        if (args.Length > 2)
        {
            File.WriteAllLines(args[2], names);
            output.WriteLine($"{names.Count} files");
            return true;
        }
        foreach (var name in names) output.WriteLine(name);
        return true;
    }

    private static bool JsonMany(DefaultFileProvider provider, string list, string outDir, TextWriter output)
    {
        var ok = true;
        foreach (var path in Lines(list))
        {
            var file = Provider.Find(provider, path);
            if (file == null)
            {
                output.WriteLine($"MISSING	{path}");
                ok = false;
                continue;
            }
            try
            {
                var package = provider.LoadPackage(file);
                var json = JsonConvert.SerializeObject(package.GetExports(), Formatting.Indented);
                var rel = file.PathWithoutExtension.ToLowerInvariant().Replace('\\', '/') + ".json";
                WriteAtomic(Path.Combine(outDir, rel), System.Text.Encoding.UTF8.GetBytes(json));
                output.WriteLine($"OK	{path}	{rel}");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL	{path}	{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }

    private static bool Textures(DefaultFileProvider provider, string list, string outDir, int max, TextWriter output)
    {
        var options = new ExportOptions(textureFormat: ETextureFormat.Png);
        Directory.CreateDirectory(outDir);
        var ok = true;
        foreach (var path in Lines(list))
        {
            var file = Provider.Find(provider, path);
            if (file == null)
            {
                output.WriteLine($"MISSING	{path}");
                ok = false;
                continue;
            }
            try
            {
                var stem = file.NameWithoutExtension.ToLowerInvariant();
                var png = Path.Combine(outDir, stem + ".png");
                if (File.Exists(png))
                {
                    output.WriteLine($"OK	{path}	cached");
                    continue;
                }
                var objectPath = file.PathWithoutExtension + "." + file.NameWithoutExtension;
                if (provider.LoadPackageObject<UTexture>(objectPath) is not { } texture) throw new Exception("not a texture");
                var bitmap = texture.Decode(max, options) ?? throw new Exception("decode returned nothing");
                var bytes = bitmap.Encode(options, out _);
                WriteAtomic(png, bytes);
                File.WriteAllText(Path.Combine(outDir, stem + ".txt"),
                    $"{texture.Format} {bitmap.Width}x{bitmap.Height} srgb={texture.SRGB} compression={texture.CompressionSettings} source={file.PathWithoutExtension}\n");
                output.WriteLine($"OK	{path}	{bitmap.Width}x{bitmap.Height}");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL	{path}	{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }

    private static bool Export(DefaultFileProvider provider, string packages, string outDir, string format, TextWriter output)
    {
        var options = new ExportOptions(
            meshFormat: format.Equals("ActorX", StringComparison.OrdinalIgnoreCase) ? EMeshFormat.ActorX : EMeshFormat.Gltf2,
            meshQuality: EMeshQuality.All,
            exportMaterials: false,
            exportMorphTargets: false);
        var session = new ExportSession();
        var ok = true;
        foreach (var path in packages.Split(';', StringSplitOptions.RemoveEmptyEntries))
        {
            var file = Provider.Find(provider, path);
            if (file == null)
            {
                output.WriteLine($"MISSING	{path}");
                ok = false;
                continue;
            }
            try
            {
                foreach (var export in provider.LoadPackage(file).GetExports())
                {
                    if (export.ExportType is "SkeletalMesh" or "StaticMesh") session.Add(export);
                }
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL	{path}	{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        Directory.CreateDirectory(outDir);
        var results = session.RunAsync(outDir, options).GetAwaiter().GetResult();
        foreach (var result in results)
        {
            output.WriteLine(result.Success ? $"OK	{result.ObjectPath}" : $"FAIL	{result.ObjectPath}");
            ok &= result.Success;
        }
        return ok;
    }

    private static bool Raw(DefaultFileProvider provider, string list, string outDir, TextWriter output)
    {
        var ok = true;
        foreach (var path in Lines(list))
        {
            if (!provider.Files.TryGetValue(path.Replace('\\', '/'), out var file))
            {
                output.WriteLine($"MISSING	{path}");
                ok = false;
                continue;
            }
            WriteAtomic(Path.Combine(outDir, file.Path), file.Read());
            output.WriteLine($"OK	{path}");
        }
        return ok;
    }

    private static void WriteAtomic(string target, byte[] bytes)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        var temp = target + ".tmp";
        File.WriteAllBytes(temp, bytes);
        File.Move(temp, target, true);
    }
}
