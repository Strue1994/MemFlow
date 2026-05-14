param(
  [Parameter(Mandatory=$true)][string]$DocxPath,
  [int]$Head = 40,
  [int]$Key = 80,
  [string]$Regex = '模块|任务|验收|阶段|里程碑|WBS|前端|后端|API|数据库|学习|引擎|流程|权限|看板|OpenCode|MemFlow|测试|部署'
)

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($DocxPath)
try {
  $entry = $zip.Entries | Where-Object { $_.FullName -eq 'word/document.xml' }
  if (-not $entry) {
    Write-Output "[ERR] word/document.xml not found: $DocxPath"
    exit 1
  }

  $sr = New-Object System.IO.StreamReader($entry.Open())
  $xmlRaw = $sr.ReadToEnd()
  $sr.Close()

  [xml]$doc = $xmlRaw
  $ns = New-Object System.Xml.XmlNamespaceManager($doc.NameTable)
  $ns.AddNamespace('w', 'http://schemas.openxmlformats.org/wordprocessingml/2006/main')

  $paras = $doc.SelectNodes('//w:p', $ns)
  $lines = @()
  foreach ($p in $paras) {
    $texts = $p.SelectNodes('.//w:t', $ns)
    if ($texts.Count -gt 0) {
      $line = ($texts | ForEach-Object { $_.'#text' }) -join ''
      if (-not [string]::IsNullOrWhiteSpace($line)) {
        $lines += $line.Trim()
      }
    }
  }

  Write-Output ("===== " + [System.IO.Path]::GetFileName($DocxPath) + " =====")
  $lines | Select-Object -First $Head
  Write-Output '--- KEY ---'
  $lines | Where-Object { $_ -match $Regex } | Select-Object -First $Key
}
finally {
  $zip.Dispose()
}
