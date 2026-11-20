using System.Text;
using CUE4Parse.FileProvider;
using CUE4Parse.UE4.Assets.Exports.Engine;
using CUE4Parse.UE4.Assets.Exports.Texture;
using CUE4Parse_Conversion;
using CUE4Parse_Conversion.Dto;
using CUE4Parse_Conversion.Options;
using CUE4Parse_Conversion.Textures;

namespace R6ToolParts;

/// <summary>The rider gear modes: skinned meshes as <c>R6RM</c> and pictures as <c>R6TX</c>
/// (docs/formats/ride6-rider-gear.md). List files hold one package per line, optionally followed by a tab and the name
/// (without extension) of the output file.</summary>
public static class Gear
{
    private const string TextureMagic = "R6TX";
    private const string MeshMagic = "R6RM";

    /// <summary>The lines of a list file: the package and the output name (default: the package's own name, lower case).</summary>
    private static IEnumerable<(string Package, string Name)> Entries(string listFile) =>
        File.ReadAllLines(listFile)
            .Select(l => l.Trim())
            .Where(l => l.Length > 0 && !l.StartsWith('#'))
            .Select(l =>
            {
                var parts = l.Split('\t');
                var package = parts[0];
                var name = parts.Length > 1 && parts[1].Length > 0
                    ? parts[1]
                    : Path.GetFileNameWithoutExtension(package.Split('.')[0]).ToLowerInvariant();
                return (package, name);
            });

    private static void WriteString(BinaryWriter w, string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        w.Write((ushort)bytes.Length);
        w.Write(bytes);
    }

    /// <summary>Each skeletal mesh as <c>R6RM</c>: the mesh's bind pose, its materials, the levels of detail in Unreal's axes.</summary>
    public static bool Meshes(DefaultFileProvider provider, string list, string outDir, TextWriter output)
    {
        Directory.CreateDirectory(outDir);
        var options = new ExportOptions(meshQuality: EMeshQuality.All, exportMaterials: false, exportMorphTargets: false);
        var ok = true;
        foreach (var (package, name) in Entries(list))
        {
            var target = Path.Combine(outDir, name + ".r6rm");
            if (File.Exists(target))
            {
                output.WriteLine($"OK\t{package}\tcached");
                continue;
            }
            var file = Provider.Find(provider, package);
            if (file == null)
            {
                output.WriteLine($"MISSING\t{package}");
                ok = false;
                continue;
            }
            try
            {
                var asset = provider.LoadPackage(file).GetExports().OfType<USkinnedAsset>().FirstOrDefault()
                    ?? throw new Exception("not a skeletal mesh");
                var dto = new SkeletalMeshDto(asset, options);
                var bytes = EncodeMesh(asset.Name, dto);
                WriteAtomic(target, bytes);
                output.WriteLine($"OK\t{package}\t{dto.LODs.Count} lods\t{dto.LODs.FirstOrDefault()?.Vertices.Length ?? 0} vertices");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL\t{package}\t{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }

    private static byte[] EncodeMesh(string name, SkeletalMeshDto dto)
    {
        using var stream = new MemoryStream();
        using var w = new BinaryWriter(stream);
        w.Write(Encoding.ASCII.GetBytes(MeshMagic));
        w.Write(1u);
        WriteString(w, name);
        w.Write((uint)dto.Bones.Length);
        foreach (var bone in dto.Bones)
        {
            WriteString(w, bone.Name);
            w.Write(bone.ParentIndex);
            var t = bone.Transform;
            foreach (var v in new[] { t.Rotation.X, t.Rotation.Y, t.Rotation.Z, t.Rotation.W }) w.Write((float)v);
            foreach (var v in new[] { t.Translation.X, t.Translation.Y, t.Translation.Z }) w.Write((float)v);
            foreach (var v in new[] { t.Scale3D.X, t.Scale3D.Y, t.Scale3D.Z }) w.Write((float)v);
        }
        w.Write((uint)dto.Materials.Length);
        foreach (var material in dto.Materials)
        {
            WriteString(w, material.SlotName);
            WriteString(w, material.Material is { IsNull: false } ? material.Material.Name : "None");
        }
        w.Write((uint)dto.LODs.Count);
        foreach (var lod in dto.LODs)
        {
            w.Write(lod.ScreenSize);
            w.Write((uint)lod.Vertices.Length);
            w.Write((uint)lod.Indices.Length);
            w.Write((uint)lod.Sections.Length);
            foreach (var section in lod.Sections)
            {
                w.Write(section.MaterialIndex);
                w.Write((uint)section.FirstIndex);
                w.Write((uint)section.NumFaces);
            }
            foreach (var vertex in lod.Vertices)
            {
                w.Write((float)vertex.Position.X);
                w.Write((float)vertex.Position.Y);
                w.Write((float)vertex.Position.Z);
                w.Write((float)vertex.Normal.X);
                w.Write((float)vertex.Normal.Y);
                w.Write((float)vertex.Normal.Z);
                w.Write((float)vertex.Uv.U);
                w.Write((float)vertex.Uv.V);
                var influences = vertex.Influences;
                var used = Math.Min(influences.Length, 8);
                w.Write((byte)used);
                for (var i = 0; i < used; i++)
                {
                    w.Write(influences[i].Bone);
                    w.Write(influences[i].Weight);
                }
            }
            foreach (var index in lod.Indices) w.Write(index);
        }
        w.Flush();
        return stream.ToArray();
    }

    /// <summary>Each texture as <c>R6TX</c>: the block-compressed mip levels as the game stores them (no decoding, no
    /// re-encoding) from the largest not above <c>max</c> pixels down, or a PNG for any other format.</summary>
    public static bool Textures(DefaultFileProvider provider, string list, string outDir, int max, TextWriter output)
    {
        Directory.CreateDirectory(outDir);
        var ok = true;
        foreach (var (package, name) in Entries(list))
        {
            var target = Path.Combine(outDir, name + ".r6tx");
            if (File.Exists(target))
            {
                output.WriteLine($"OK\t{package}\tcached");
                continue;
            }
            var file = Provider.Find(provider, package);
            if (file == null)
            {
                output.WriteLine($"MISSING\t{package}");
                ok = false;
                continue;
            }
            try
            {
                var objectPath = file.PathWithoutExtension + "." + file.NameWithoutExtension;
                if (provider.LoadPackageObject<UTexture>(objectPath) is not { } texture) throw new Exception("not a texture");
                var (bytes, summary) = EncodeTexture(texture, max);
                WriteAtomic(target, bytes);
                output.WriteLine($"OK\t{package}\t{summary}");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL\t{package}\t{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }

    private static uint FormatCode(EPixelFormat format) => format switch
    {
        EPixelFormat.PF_DXT1 => 1,
        EPixelFormat.PF_DXT3 => 2,
        EPixelFormat.PF_DXT5 => 3,
        _ => 0xFF,
    };

    private static (byte[] Bytes, string Summary) EncodeTexture(UTexture texture, int max)
    {
        using var stream = new MemoryStream();
        using var w = new BinaryWriter(stream);
        w.Write(Encoding.ASCII.GetBytes(TextureMagic));
        w.Write(1u);
        var code = texture.PlatformData.VTData != null ? 0xFFu : FormatCode(texture.Format);
        w.Write(code);
        w.Write((byte)(texture.SRGB ? 1 : 0));
        if (code == 0xFF)
        {
            // Another codec (or a virtual texture): decoded by CUE4Parse, kept as one PNG.
            var options = new ExportOptions(textureFormat: ETextureFormat.Png);
            var bitmap = texture.Decode(max, options) ?? throw new Exception("decode returned nothing");
            var png = bitmap.Encode(options, out _);
            w.Write(1u);
            w.Write((uint)bitmap.Width);
            w.Write((uint)bitmap.Height);
            w.Write((uint)png.Length);
            w.Write(png);
            w.Flush();
            return (stream.ToArray(), $"png {bitmap.Width}x{bitmap.Height} from {texture.Format}");
        }
        // The first mip level with both sides within `max` (the smallest one when none is).
        var first = -1;
        for (var i = 0; i < texture.PlatformData.Mips.Length && first < 0; i++)
        {
            var mip = texture.PlatformData.Mips[i];
            if (Math.Max(mip.SizeX, mip.SizeY) <= max) first = i;
        }
        if (first < 0) first = texture.PlatformData.Mips.Length - 1;
        if (first < 0) throw new Exception("no mip level has data");
        var mips = new List<(int W, int H, byte[] Data)>();
        for (var i = first; i < texture.PlatformData.Mips.Length; i++)
        {
            var mip = texture.GetMip(i);
            if (mip?.BulkData?.Data is not { } data) break;
            mips.Add((mip.SizeX, mip.SizeY, data));
        }
        if (mips.Count == 0) throw new Exception("no mip data");
        w.Write((uint)mips.Count);
        foreach (var (width, height, data) in mips)
        {
            w.Write((uint)width);
            w.Write((uint)height);
            w.Write((uint)data.Length);
            w.Write(data);
        }
        w.Flush();
        return (stream.ToArray(), $"{texture.Format} {mips[0].W}x{mips[0].H} {mips.Count} mips");
    }

    private static void WriteAtomic(string target, byte[] bytes)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(target)!);
        var temp = target + ".tmp";
        File.WriteAllBytes(temp, bytes);
        File.Move(temp, target, true);
    }
}
