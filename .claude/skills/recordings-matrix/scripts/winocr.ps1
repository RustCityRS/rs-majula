# OCR every PNG in a directory with Windows' built-in engine (Windows.Media.Ocr), one TSV line each:
# <file name> TAB <line 1> / <line 2> / ...
# Windows PowerShell 5.1 only (WinRT projection): powershell.exe -NoProfile -File winocr.ps1 <dir> <out.tsv> [scale]
param([string]$Dir, [string]$Out, [double]$Scale = 2.0)
Add-Type -AssemblyName System.Runtime.WindowsRuntime
Add-Type -AssemblyName System.Drawing
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics, ContentType = WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Globalization, ContentType = WindowsRuntime]
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($op, $type) {
    $t = $asTaskGeneric.MakeGenericMethod($type).Invoke($null, @($op))
    $t.Wait(-1) | Out-Null
    $t.Result
}
$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en-US'))
if ($null -eq $engine) { $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages() }
$tmp = Join-Path $Dir '_ocr_tmp.png'
$lines = New-Object System.Collections.Generic.List[string]
Get-ChildItem -Path $Dir | Where-Object { $_.Name -match '^\d.*\.(png|jpg)$' } | Sort-Object Name | ForEach-Object {
    # Upscale first: the engine reads small game fonts far better at 2x.
    $src = [System.Drawing.Image]::FromFile($_.FullName)
    # Low-resolution recordings get more: every crop ends up about 1400 px wide.
    $s = [Math]::Max($Scale, 1400.0 / $src.Width)
    $w = [int]($src.Width * $s); $h = [int]($src.Height * $s)
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($src, 0, 0, $w, $h)
    $g.Dispose(); $src.Dispose()
    $bmp.Save($tmp, [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($tmp)) ([Windows.Storage.StorageFile])
    $stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
    $decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
    $result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
    $stream.Dispose()
    $text = ($result.Lines | ForEach-Object { $_.Text }) -join ' / '
    $lines.Add("$($_.Name)`t$text")
}
Remove-Item $tmp -ErrorAction SilentlyContinue
[System.IO.File]::WriteAllLines($Out, $lines)
"$($lines.Count) images"
