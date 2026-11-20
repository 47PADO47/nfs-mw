using CUE4Parse.FileProvider;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace R6ToolSnd;

/// <summary>Mode <c>audiomap &lt;folder&gt;</c>: one <c>&lt;chassis&gt;__&lt;variant&gt;.json</c> per row of the chassis audio
/// table (<c>assets/audio/Bindings/bikes/CHASSIS_AUDIO</c>, rows <c>CH_&lt;chassis&gt;$Stock|Custom</c>), the form
/// <c>mods/ride6-bikes/src/sound/data.rs</c> reads: the Wwise events of the bike, with the banks and media each lists, and
/// the numbers of the data asset's <c>AudioEffects</c> next to those of the base asset (the importer merges them, as the
/// game's property inheritance does; the layout is in <c>docs/formats/ride6-audio.md</c>).</summary>
public static class AudioMap
{
    private const string Bindings = "/Game/assets/audio/Bindings/Bikes/";
    private static readonly string[] EventRoles =
    {
        "EngineEvent", "ExhaustEvent", "BackfireBangEvent", "BackfireFlameEvent",
        "GearUpChangeEvent", "GearDownChangeEvent", "SuperchargerEvent",
    };

    public static int Run(DefaultFileProvider provider, string outDir)
    {
        var packages = new Packages(provider);
        var table = packages.Load(Bindings + "CHASSIS_AUDIO");
        var rows = table == null ? null : Packages.Export(table, "DataTable")?["Rows"] as JObject;
        if (rows == null)
        {
            Console.Error.WriteLine("the chassis audio table is not in the install");
            return 1;
        }
        var baseAsset = packages.Load(Bindings + "BikeAudioDataSource");
        var baseProps = baseAsset == null ? null : Packages.Export(baseAsset, "BikeAudioDataSource_C")?["Properties"] as JObject;
        if (baseProps == null)
        {
            Console.Error.WriteLine("the base audio data asset is not in the install");
            return 1;
        }
        Directory.CreateDirectory(outDir);
        var events = new Dictionary<string, JToken?>(StringComparer.OrdinalIgnoreCase);
        var failed = 0;
        var written = 0;
        foreach (var row in rows.Properties())
        {
            try
            {
                var document = Resolve(packages, events, baseProps, row);
                if (document == null)
                {
                    Console.Error.WriteLine($"{row.Name}: the data asset is not in the install");
                    failed++;
                    continue;
                }
                File.WriteAllText(Path.Combine(outDir, $"{document["chassis"]}__{document["variant"]}.json"),
                    document.ToString(Formatting.Indented));
                written++;
            }
            catch (Exception e)
            {
                Console.Error.WriteLine($"{row.Name}: {e.GetType().Name}: {e.Message}");
                failed++;
            }
        }
        Console.WriteLine($"{written} rows written, {failed} failed, {events.Count} events read");
        return failed == 0 ? 0 : 1;
    }

    /// <summary>The document of one row, or null when its data asset is not in the install.</summary>
    private static JObject? Resolve(Packages packages, Dictionary<string, JToken?> events, JObject baseProps, JProperty row)
    {
        var name = row.Name;
        var dollar = name.LastIndexOf('$');
        if (!name.StartsWith("CH_") || dollar < 0) throw new InvalidDataException("unexpected row name");
        var chassis = name[3..dollar];
        var variant = name[(dollar + 1)..];
        var assetPath = (string?)row.Value["DataAsset"]?["AssetPathName"];
        if (string.IsNullOrEmpty(assetPath) || assetPath == "None") return null;
        var exports = packages.Load(assetPath);
        var props = exports == null ? null : Packages.Export(exports, "BikeAudioDataSource_C")?["Properties"] as JObject;
        if (props == null) return null;

        var resolved = new JObject();
        foreach (var role in EventRoles)
        {
            // A key the data asset sets, even to null (no event: `GearUpChangeEvent` of some stock bikes), wins over the base.
            var reference = props.ContainsKey(role) ? props[role] : baseProps[role];
            var path = reference is JObject target ? (string?)target["ObjectPath"] : null;
            resolved[role] = path == null ? JValue.CreateNull() : Event(packages, events, path) ?? JValue.CreateNull();
        }
        var dot = assetPath.LastIndexOf('.');
        return new JObject
        {
            ["chassis"] = chassis,
            ["variant"] = variant,
            ["data_asset"] = dot < 0 ? assetPath : assetPath[..dot],
            ["events"] = resolved,
            ["audio_effects_default"] = baseProps["AudioEffects"] ?? new JObject(),
            ["audio_effects_override"] = props["AudioEffects"] ?? JValue.CreateNull(),
        };
    }

    /// <summary>The event of an object path as <c>data.rs</c> reads it, each event read once.</summary>
    private static JToken? Event(Packages packages, Dictionary<string, JToken?> events, string path)
    {
        if (events.TryGetValue(path, out var known)) return known?.DeepClone();
        JToken? result = null;
        var exports = packages.Load(path);
        var export = exports == null ? null : Packages.Export(exports, "AkAudioEvent");
        var languages = export?["EventCookedData"]?["EventLanguageMap"] as JArray;
        var entry = languages?.FirstOrDefault(language => (string?)language["Key"]?["LanguageName"] == "SFX")
            ?? languages?.FirstOrDefault();
        if (export != null && entry?["Value"] is JObject value) result = Describe(export, value);
        events[path] = result;
        return result?.DeepClone();
    }

    private static JObject Describe(JToken export, JObject value)
    {
        var banks = new JArray();
        foreach (var bank in value["SoundBanks"] ?? new JArray())
        {
            var kind = (string?)bank["SoundBankType"] ?? "";
            var colons = kind.LastIndexOf("::", StringComparison.Ordinal);
            banks.Add(new JObject
            {
                ["name"] = (string?)bank["DebugName"] ?? "",
                ["id"] = (uint?)bank["SoundBankId"] ?? 0,
                ["file"] = (string?)bank["PackagedFile"]?["PathName"] ?? "",
                ["has_media"] = (bool?)bank["bContainsMedia"] ?? false,
                ["kind"] = colons < 0 ? kind : kind[(colons + 2)..],
            });
        }
        var media = new JArray();
        foreach (var item in value["Media"] ?? new JArray())
        {
            media.Add(new JObject
            {
                ["id"] = (uint?)item["MediaId"] ?? 0,
                ["name"] = (string?)item["DebugName"] ?? "",
                ["file"] = (string?)item["PackagedFile"]?["PathName"] ?? "",
            });
        }
        return new JObject
        {
            ["name"] = (string?)export["Name"] ?? "",
            ["event_id"] = (uint?)value["EventId"] ?? 0,
            ["banks"] = banks,
            ["media"] = media,
            ["min_duration_s"] = (float?)export["MinimumDuration"],
            ["max_duration_s"] = (float?)export["MaximumDuration"],
            ["infinite"] = (bool?)export["IsInfinite"],
        };
    }
}
