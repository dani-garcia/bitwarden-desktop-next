param(
    [string]$OutputPath = "img\screenshot.png",
    [string]$WindowTitle = "Bitwarden"
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
using System.Drawing.Imaging;

public class WindowCapture {
    [DllImport("user32.dll")]
    public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr hWnd);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left, Top, Right, Bottom;
    }

    public static void Capture(string title, string path) {
        IntPtr hwnd = FindWindow(null, title);
        if (hwnd == IntPtr.Zero) {
            Console.WriteLine("Window not found: " + title);
            return;
        }
        SetForegroundWindow(hwnd);
        System.Threading.Thread.Sleep(300);
        RECT rect;
        GetWindowRect(hwnd, out rect);
        int w = rect.Right - rect.Left;
        int h = rect.Bottom - rect.Top;
        using (Bitmap bmp = new Bitmap(w, h)) {
            using (Graphics g = Graphics.FromImage(bmp)) {
                g.CopyFromScreen(rect.Left, rect.Top, 0, 0, new Size(w, h));
            }
            bmp.Save(path, ImageFormat.Png);
            Console.WriteLine("Saved: " + path + " (" + w + "x" + h + ")");
        }
    }
}
"@ -ReferencedAssemblies System.Drawing

[WindowCapture]::Capture($WindowTitle, $OutputPath)
