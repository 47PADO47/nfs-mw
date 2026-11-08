using System.Text;

namespace R6ToolParts;

/// <summary>Console entry: one-shot mode (<c>R6ToolParts mode args...</c>) or <c>serve</c> (requests as tab separated
/// lines on stdin, each answered by its output followed by a line holding <see cref="Done"/>).</summary>
public static class Program
{
    public const string Done = "\u0001DONE";

    public static int Main(string[] args)
    {
        if (args.Length == 0)
        {
            Console.Error.WriteLine(Commands.Usage);
            return 2;
        }
        var provider = Provider.Open();
        if (args[0] != "serve") return Commands.Run(provider, args, Console.Out) ? 0 : 1;

        var stdin = Console.In;
        var stdout = new StreamWriter(Console.OpenStandardOutput(), new UTF8Encoding(false)) { AutoFlush = true };
        stdout.WriteLine("READY");
        while (stdin.ReadLine() is { } line)
        {
            if (line == "quit") break;
            var writer = new StringWriter();
            try
            {
                Commands.Run(provider, line.Split('\t'), writer);
            }
            catch (Exception e)
            {
                writer.WriteLine("ERROR " + e.GetType().Name + ": " + e.Message);
            }
            stdout.Write(writer.ToString());
            stdout.WriteLine(Done);
        }
        return 0;
    }
}
