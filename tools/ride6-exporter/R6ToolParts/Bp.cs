using CUE4Parse.FileProvider;
using CUE4Parse.UE4.Kismet;
using CUE4Parse.UE4.Objects.Engine;
using CUE4Parse.UE4.Objects.UObject;
using CUE4Parse.UE4.Objects.UObject.BlueprintDecompiler;

namespace R6ToolParts;

/// <summary>The <c>bp</c> mode: a Blueprint class as the pseudo-C++ CUE4Parse's decompiler writes (variables, defaults and
/// the functions' expressions), used to read the animation blueprint of the rider (docs/specs/ride6-rider-styles.md).</summary>
public static class Bp
{
    /// <summary>Each listed package's Blueprint classes as <c>&lt;outdir&gt;/&lt;class&gt;.cpp</c>.</summary>
    public static bool Classes(DefaultFileProvider provider, string list, string outDir, TextWriter output)
    {
        Directory.CreateDirectory(outDir);
        provider.ReadScriptData = true; // the functions' bytecode is skipped unless asked for
        var ok = true;
        foreach (var line in File.ReadAllLines(list).Select(l => l.Trim()).Where(l => l.Length > 0 && !l.StartsWith('#')))
        {
            var file = Provider.Find(provider, line);
            if (file == null)
            {
                output.WriteLine($"MISSING\t{line}");
                ok = false;
                continue;
            }
            try
            {
                var package = provider.LoadPackage(file);
                foreach (var export in package.GetExports())
                {
                    if (export is not UBlueprintGeneratedClass cls) continue;
                    var target = Path.Combine(outDir, export.Name + ".cpp");
                    File.WriteAllText(target, cls.DecompileBlueprintToPseudo());
                    File.WriteAllText(Path.Combine(outDir, export.Name + ".functions.txt"), Functions(cls));
                    output.WriteLine($"OK\t{line}\t{target}");
                }
            }
            catch (Exception e)
            {
                output.WriteLine($"FAIL\t{line}\t{e.GetType().Name}: {e.Message}");
                ok = false;
            }
        }
        return ok;
    }

    /// <summary>The bodies of the class's functions, one statement per line behind its offset. CUE4Parse's own decompiler stops at
    /// the first function without bytecode; this one goes on.</summary>
    private static string Functions(UClass cls)
    {
        BlueprintDecompilerUtils.Mappings = cls.Owner?.Mappings;
        var text = new System.Text.StringBuilder();
        foreach (var (name, reference) in cls.FuncMap)
        {
            if (!reference.TryLoad(out var export) || export is not UFunction function) continue;
            text.AppendLine($"// {function.FunctionFlags}");
            text.AppendLine($"function {name.Text}");
            BlueprintDecompilerUtils.Function = function;
            if (function.ScriptBytecode == null || function.ScriptBytecode.Length == 0)
            {
                text.AppendLine("  // no bytecode");
                continue;
            }
            foreach (var expression in function.ScriptBytecode)
            {
                if (expression is EX_Nothing or EX_NothingInt32 or EX_EndFunctionParms or EX_EndStructConst or EX_EndArray
                    or EX_EndArrayConst or EX_EndSet or EX_EndMap or EX_EndMapConst or EX_EndSetConst or EX_EndOfScript) continue;
                string line;
                try
                {
                    line = BlueprintDecompilerUtils.GetLineExpression(expression);
                }
                catch (Exception e)
                {
                    line = "ERROR " + e.GetType().Name;
                }
                if (!string.IsNullOrWhiteSpace(line)) text.AppendLine($"  [{expression.StatementIndex}] {line};");
            }
            text.AppendLine();
        }
        return text.ToString();
    }
}
