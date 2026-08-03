# Gera o icone do FzComputerAI: fundo preto + circulo rugoso branco no centro.
# Saidas:
#   installer\fzcomputerai.ico          -> icone do .exe (build.rs) e do instalador
#   fzcomputerai\assets\icon64.rgba     -> RGBA cru p/ o icone da JANELA (with_icon),
#                                          embutido por include_bytes! (sem dependencia nova)
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

$repo = 'G:\fzcomcontrol'
$S = 256

function New-IconBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Black)

    $cx = $size / 2.0
    $cy = $size / 2.0
    $rOut = $size * 0.40          # raio externo do anel
    $rIn  = $size * 0.115         # buraco central
    $rand = New-Object System.Random(20260802)   # deterministico

    # --- anel externo irregular (rugoso), branco ---
    $pts = New-Object System.Collections.Generic.List[System.Drawing.PointF]
    $steps = 96
    for ($i = 0; $i -lt $steps; $i++) {
        $a = 2 * [Math]::PI * $i / $steps
        # ondulacao: 2 harmonicos + ruido leve => borda "rugosa"
        $wob = 1.0 + 0.055 * [Math]::Sin(9 * $a) + 0.035 * [Math]::Sin(14 * $a + 1.2) + ($rand.NextDouble() - 0.5) * 0.028
        $r = $rOut * $wob
        $pts.Add((New-Object System.Drawing.PointF([float]($cx + $r * [Math]::Cos($a)), [float]($cy + $r * [Math]::Sin($a)))))
    }
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddClosedCurve($pts.ToArray(), 0.25)
    $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 245, 245, 245))
    $g.FillPath($white, $path)

    # --- pregas radiais (o "rugoso"): linhas escuras do centro para a borda ---
    $nFolds = 26
    for ($i = 0; $i -lt $nFolds; $i++) {
        $a = 2 * [Math]::PI * $i / $nFolds + 0.05
        $jit = ($rand.NextDouble() - 0.5) * 0.10
        $a2 = $a + $jit
        $r1 = $rIn * 1.05
        $r2 = $rOut * (0.90 + ($rand.NextDouble() - 0.5) * 0.14)
        $x1 = $cx + $r1 * [Math]::Cos($a);  $y1 = $cy + $r1 * [Math]::Sin($a)
        $x2 = $cx + $r2 * [Math]::Cos($a2); $y2 = $cy + $r2 * [Math]::Sin($a2)
        # curva leve, largura variavel: parece prega, nao raio de roda
        $w = [float]([Math]::Max(1.0, $size * (0.012 + ($rand.NextDouble() * 0.010))))
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(210, 25, 25, 25)), $w
        $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $mx = ($x1 + $x2) / 2 + ($rand.NextDouble() - 0.5) * $size * 0.03
        $my = ($y1 + $y2) / 2 + ($rand.NextDouble() - 0.5) * $size * 0.03
        $g.DrawCurve($pen, [System.Drawing.PointF[]]@(
            (New-Object System.Drawing.PointF([float]$x1, [float]$y1)),
            (New-Object System.Drawing.PointF([float]$mx, [float]$my)),
            (New-Object System.Drawing.PointF([float]$x2, [float]$y2))
        ), 0.4)
        $pen.Dispose()
    }

    # --- buraco central escuro ---
    $holePts = New-Object System.Collections.Generic.List[System.Drawing.PointF]
    for ($i = 0; $i -lt 48; $i++) {
        $a = 2 * [Math]::PI * $i / 48
        $wob = 1.0 + 0.13 * [Math]::Sin(7 * $a + 0.6) + ($rand.NextDouble() - 0.5) * 0.06
        $r = $rIn * $wob
        $holePts.Add((New-Object System.Drawing.PointF([float]($cx + $r * [Math]::Cos($a)), [float]($cy + $r * [Math]::Sin($a)))))
    }
    $holePath = New-Object System.Drawing.Drawing2D.GraphicsPath
    $holePath.AddClosedCurve($holePts.ToArray(), 0.3)
    $dark = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 10, 10, 10))
    $g.FillPath($dark, $holePath)

    $white.Dispose(); $dark.Dispose(); $path.Dispose(); $holePath.Dispose(); $g.Dispose()
    return $bmp
}

$master = New-IconBitmap $S

# ---------- .ico multi-tamanho (PNG dentro do ICO, suportado desde Vista) ----------
$sizes = @(16, 32, 48, 64, 128, 256)
$pngs = @()
foreach ($sz in $sizes) {
    $b = New-Object System.Drawing.Bitmap($master, $sz, $sz)
    $ms = New-Object System.IO.MemoryStream
    $b.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngs += ,@{ size = $sz; bytes = $ms.ToArray() }
    $ms.Dispose(); $b.Dispose()
}

$icoPath = Join-Path $repo 'installer\fzcomputerai.ico'
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)
$bw.Write([UInt16]0)                 # reserved
$bw.Write([UInt16]1)                 # type = icon
$bw.Write([UInt16]$pngs.Count)       # count
$offset = 6 + (16 * $pngs.Count)
foreach ($p in $pngs) {
    $dim = if ($p.size -ge 256) { 0 } else { $p.size }
    $bw.Write([Byte]$dim)            # width
    $bw.Write([Byte]$dim)            # height
    $bw.Write([Byte]0)               # palette
    $bw.Write([Byte]0)               # reserved
    $bw.Write([UInt16]1)             # color planes
    $bw.Write([UInt16]32)            # bpp
    $bw.Write([UInt32]$p.bytes.Length)
    $bw.Write([UInt32]$offset)
    $offset += $p.bytes.Length
}
foreach ($p in $pngs) { $bw.Write($p.bytes) }
$bw.Flush(); $bw.Close(); $fs.Close()

# ---------- RGBA cru 64x64 para o icone da JANELA (eframe with_icon) ----------
$w64 = New-Object System.Drawing.Bitmap($master, 64, 64)
$rgba = New-Object System.Collections.Generic.List[Byte]
for ($y = 0; $y -lt 64; $y++) {
    for ($x = 0; $x -lt 64; $x++) {
        $c = $w64.GetPixel($x, $y)
        $rgba.Add($c.R); $rgba.Add($c.G); $rgba.Add($c.B); $rgba.Add($c.A)
    }
}
$assetsDir = Join-Path $repo 'fzcomputerai\assets'
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null
[System.IO.File]::WriteAllBytes((Join-Path $assetsDir 'icon64.rgba'), $rgba.ToArray())
$w64.Dispose()

# PNG de preview para conferir visualmente
$previewPath = Join-Path $env:TEMP 'fz-icon-preview.png'
$master.Save($previewPath, [System.Drawing.Imaging.ImageFormat]::Png)
$master.Dispose()

Write-Output "ICO:     $icoPath ($((Get-Item $icoPath).Length) bytes, $($pngs.Count) tamanhos)"
Write-Output "RGBA64:  $(Join-Path $assetsDir 'icon64.rgba') ($((Get-Item (Join-Path $assetsDir 'icon64.rgba')).Length) bytes = 64*64*4)"
Write-Output "PREVIEW: $previewPath"
