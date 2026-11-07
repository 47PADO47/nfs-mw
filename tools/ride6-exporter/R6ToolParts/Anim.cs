using System.Text;
using CUE4Parse.FileProvider;
using CUE4Parse.UE4.Assets.Exports.Animation;
using CUE4Parse.UE4.Assets.Exports.Animation.ACL;
using CUE4Parse.UE4.Assets.Exports.SkeletalMesh;
using CUE4Parse.UE4.Objects.Core.Math;
using CUE4Parse_Conversion.Animations;
using Newtonsoft.Json;

namespace R6ToolParts;

/// <summary>The animation modes (docs/formats/ride6-rider.md): the rider skeleton as JSON, animation sequences as <c>R6RA</c>
/// and the ACL-compressed ones raw with a small meta file, as the importer <c>ride6-rider-import</c> reads them.</summary>
public static class Anim
{
    private static IEnumerable<string> Lines(string listFile) =>
        File.ReadAllLines(listFile).Select(l => l.Trim()).Where(l => l.Length > 0 && !l.StartsWith('#'));

    private static void WriteString(BinaryWriter w, string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        w.Write((ushort)bytes.Length);
        w.Write(bytes);
    }

    private static UAnimSequence? Sequence(DefaultFileProvider provider, string path, TextWriter output)
    {
        var file = Provider.Find(provider, path);
        if (file == null)
        {
            output.WriteLine($"MISSING\t{path}");
            return null;
        }
        var package = provider.LoadPackage(file);
        var sequence = package.GetExports().OfType<UAnimSequence>().FirstOrDefault();
        if (sequence == null) output.WriteLine($"NOSEQUENCE\t{path}");
        return sequence;
    }

    /// <summary>The skeleton of the first sequence of <paramref name="package"/> (or the package's own skeleton, the bike's
    /// <c>chassis369_Skeleton</c> for example) as <c>skeleton.json</c>.</summary>
    public static bool Skeleton(DefaultFileProvider provider, string package, string outFile, TextWriter output)
    {
        USkeleton? skeleton = null;
        var file = Provider.Find(provider, package);
        if (file != null) skeleton = provider.LoadPackage(file).GetExports().OfType<USkeleton>().FirstOrDefault();
        if (skeleton == null)
        {
            var sequence = Sequence(provider, package, output);
            if (sequence?.Skeleton == null || !sequence.Skeleton.TryLoad<USkeleton>(out skeleton)) return false;
        }
        var reference = skeleton.ReferenceSkeleton;
        var bones = new List<object>();
        for (var i = 0; i < skeleton.BoneCount; i++)
        {
            var pose = reference.FinalRefBonePose[i];
            bones.Add(new
            {
                name = reference.FinalRefBoneInfo[i].Name.Text,
                parent = reference.FinalRefBoneInfo[i].ParentIndex,
                t = new[] { pose.Translation.X, pose.Translation.Y, pose.Translation.Z },
                q = new[] { pose.Rotation.X, pose.Rotation.Y, pose.Rotation.Z, pose.Rotation.W },
                s = new[] { pose.Scale3D.X, pose.Scale3D.Y, pose.Scale3D.Z },
                retarget = skeleton.BoneTree.Length > i ? skeleton.BoneTree[i].ToString() : "Animation",
            });
        }
        var sockets = new List<object>();
        foreach (var index in skeleton.Sockets)
        {
            if (!index.TryLoad<USkeletalMeshSocket>(out var socket)) continue;
            sockets.Add(new
            {
                name = socket.SocketName.Text,
                bone = socket.BoneName.Text,
                t = new[] { socket.RelativeLocation.X, socket.RelativeLocation.Y, socket.RelativeLocation.Z },
                r = new[] { socket.RelativeRotation.Pitch, socket.RelativeRotation.Yaw, socket.RelativeRotation.Roll },
                s = new[] { socket.RelativeScale.X, socket.RelativeScale.Y, socket.RelativeScale.Z },
            });
        }
        var json = JsonConvert.SerializeObject(new { bones, sockets, virtual_bones = Array.Empty<object>() }, Formatting.Indented);
        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(outFile))!);
        File.WriteAllText(outFile, json);
        output.WriteLine($"OK\t{skeleton.BoneCount} bones, {sockets.Count} sockets");
        return true;
    }

    private static float NotifyTime(FAnimNotifyEvent notify, float length) =>
        notify.LinkMethod == EAnimLinkMethod.Relative ? notify.LinkValue * length : notify.LinkValue;

    /// <summary>Every listed sequence that Unreal's own codecs store, sampled at each source frame, as one <c>R6RA</c> file.
    /// ACL sequences are skipped (the <c>acl</c> mode writes those).</summary>
    public static bool Anims(DefaultFileProvider provider, string list, string outFile, TextWriter output)
    {
        var ok = true;
        var clips = new List<(UAnimSequence Sequence, FTransform[] Reference, float[][] Keys, int Frames)>();
        var bones = 0;
        foreach (var path in Lines(list))
        {
            try
            {
                var sequence = Sequence(provider, path, output);
                if (sequence?.Skeleton == null || !sequence.Skeleton.TryLoad<USkeleton>(out var skeleton))
                {
                    ok = false;
                    continue;
                }
                if (sequence.CompressedDataStructure is FACLCompressedAnimData)
                {
                    output.WriteLine($"ACL\t{path}");
                    continue;
                }
                var set = sequence.ConvertAnims();
                var track = set.Sequences[0];
                bones = skeleton.BoneCount;
                var reference = skeleton.ReferenceSkeleton.FinalRefBonePose;
                var frames = Math.Max(1, sequence.NumFrames);
                var keys = new float[frames * bones][];
                for (var f = 0; f < frames; f++)
                {
                    for (var b = 0; b < bones; b++)
                    {
                        var q = reference[b].Rotation;
                        var p = reference[b].Translation;
                        var s = reference[b].Scale3D;
                        if (b < track.Tracks.Count) track.Tracks[b].GetBoneTransform(f, frames, ref q, ref p, ref s);
                        keys[f * bones + b] = [q.X, q.Y, q.Z, q.W, p.X, p.Y, p.Z];
                    }
                }
                clips.Add((sequence, reference, keys, frames));
                output.WriteLine($"OK\t{path}\t{frames} frames");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL\t{path}\t{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(outFile))!);
        using var stream = File.Create(outFile);
        using var w = new BinaryWriter(stream);
        w.Write(Encoding.ASCII.GetBytes("R6RA"));
        w.Write(1u);
        w.Write((uint)bones);
        w.Write((uint)clips.Count);
        foreach (var (sequence, reference, keys, frames) in clips)
        {
            WriteString(w, sequence.Name);
            w.Write(sequence.SequenceLength);
            w.Write((uint)frames);
            w.Write(sequence.RateScale);
            w.Write((byte)(sequence.AdditiveAnimType == EAdditiveAnimationType.AAT_None ? 0 : sequence.AdditiveAnimType == EAdditiveAnimationType.AAT_LocalSpaceBase ? 1 : 2));
            WriteString(w, sequence.RefPoseSeq?.Name.Text ?? "");
            w.Write(sequence.RefFrameIndex);
            w.Write((byte)(sequence.bEnableRootMotion ? 1 : 0));
            var notifies = sequence.Notifies ?? [];
            w.Write((uint)notifies.Length);
            foreach (var notify in notifies)
            {
                WriteString(w, notify.NotifyName?.Text ?? "");
                w.Write(NotifyTime(notify, sequence.SequenceLength));
            }
            foreach (var pose in reference)
            {
                w.Write(pose.Scale3D.X);
                w.Write(pose.Scale3D.Y);
                w.Write(pose.Scale3D.Z);
            }
            foreach (var key in keys)
                foreach (var value in key) w.Write(value);
        }
        return ok;
    }

    /// <summary>Every listed ACL-compressed sequence as <c>&lt;name&gt;.acl</c> (the compressed buffer) and
    /// <c>&lt;name&gt;.meta.json</c> (what the sequence has besides it).</summary>
    public static bool Acl(DefaultFileProvider provider, string list, string outDir, TextWriter output)
    {
        Directory.CreateDirectory(outDir);
        var ok = true;
        foreach (var path in Lines(list))
        {
            try
            {
                var sequence = Sequence(provider, path, output);
                if (sequence == null)
                {
                    ok = false;
                    continue;
                }
                if (sequence.CompressedDataStructure is not FACLCompressedAnimData acl)
                {
                    output.WriteLine($"NOTACL\t{path}");
                    continue;
                }
                File.WriteAllBytes(Path.Combine(outDir, sequence.Name + ".acl"), acl.CompressedByteStream);
                var notifies = (sequence.Notifies ?? []).Select(n => new
                {
                    name = n.NotifyName?.Text ?? "",
                    link = n.LinkMethod.ToString(),
                    time = n.LinkValue,
                });
                var meta = new
                {
                    name = sequence.Name,
                    package = path,
                    num_frames = acl.CompressedNumberOfFrames,
                    length = sequence.SequenceLength,
                    rate_scale = sequence.RateScale,
                    additive = sequence.AdditiveAnimType.ToString(),
                    base_name = sequence.RefPoseSeq?.Name.Text ?? "",
                    base_frame = sequence.RefFrameIndex,
                    root_motion = sequence.bEnableRootMotion,
                    interpolation = sequence.Interpolation.ToString(),
                    track_to_bone = sequence.GetTrackMap().Select(t => t.BoneTreeIndex).ToArray(),
                    notifies,
                    codec = "ACL",
                };
                File.WriteAllText(Path.Combine(outDir, sequence.Name + ".meta.json"), JsonConvert.SerializeObject(meta, Formatting.Indented));
                output.WriteLine($"OK\t{path}");
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL\t{path}\t{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }
}
