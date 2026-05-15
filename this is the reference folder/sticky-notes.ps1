Add-Type -AssemblyName PresentationFramework
Add-Type -AssemblyName PresentationCore
Add-Type -AssemblyName WindowsBase

# Resolve base directory (works in both .ps1 and .exe mode)
try {
    if ($MyInvocation.MyCommand.Path -and [System.IO.Path]::IsPathRooted($MyInvocation.MyCommand.Path)) {
        $baseDir = [System.IO.Path]::GetDirectoryName($MyInvocation.MyCommand.Path)
    } else {
        $exePath = [System.Reflection.Assembly]::GetEntryAssembly().Location
        if ($exePath -and [System.IO.Path]::IsPathRooted($exePath)) {
            $baseDir = [System.IO.Path]::GetDirectoryName($exePath)
        }
    }
} catch {
    $baseDir = $null
}

if (-not $baseDir) {
    [System.Windows.MessageBox]::Show("Error: baseDir is null or invalid")
    exit
}

# Paths
$configPath = Join-Path $baseDir 'StickyNote.cfg'
$dataPath   = Join-Path $baseDir 'temp.json'

# Define slim scrollbar style for TextBox
$xaml = @"
<ResourceDictionary xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Style TargetType="TextBox">
    <Setter Property="Template">
      <Setter.Value>
        <ControlTemplate TargetType="TextBox">
          <ScrollViewer x:Name="PART_ContentHost"
              Background="{TemplateBinding Background}"
              VerticalScrollBarVisibility="Auto"
              HorizontalScrollBarVisibility="Auto"
              Padding="0">
            <ScrollViewer.Resources>
              <Style TargetType="ScrollBar">
                <Setter Property="Width" Value="4"/>
                <Setter Property="MinWidth" Value="4"/>
                <Setter Property="Template">
                  <Setter.Value>
                    <ControlTemplate TargetType="ScrollBar">
                      <Grid Background="Transparent" Margin="0">
                        <Track Name="PART_Track" IsDirectionReversed="true" Margin="0">
                          <Track.DecreaseRepeatButton>
                            <RepeatButton Command="ScrollBar.LineUpCommand" Visibility="Collapsed"/>
                          </Track.DecreaseRepeatButton>
                          <Track.IncreaseRepeatButton>
                            <RepeatButton Command="ScrollBar.LineDownCommand" Visibility="Collapsed"/>
                          </Track.IncreaseRepeatButton>
                          <Track.Thumb>
                            <Thumb Background="#30CCCCCC" BorderThickness="0" Width="4" Margin="0"/>
                          </Track.Thumb>
                        </Track>
                      </Grid>
                    </ControlTemplate>
                  </Setter.Value>
                </Setter>
              </Style>
            </ScrollViewer.Resources>
          </ScrollViewer>
        </ControlTemplate>
      </Setter.Value>
    </Setter>
  </Style>
</ResourceDictionary>
"@

$reader = New-Object System.Xml.XmlTextReader([System.IO.StringReader]$xaml)
$resourceDict = [Windows.Markup.XamlReader]::Load($reader)
$textBoxStyle = ($resourceDict.Values | Where-Object { $_.TargetType.Name -eq "TextBox" })[0]

# Load config
function Load-Config {
    if (Test-Path $configPath) {
        try {
            $cfg = Get-Content $configPath | ConvertFrom-Json
            return @{
                Width     = $cfg.Width
                Height    = $cfg.Height
                Left      = $cfg.Left
                Top       = $cfg.Top
                FontSize  = $cfg.FontSize
            }
        } catch {
            return @{ Width = 300; Height = 400; Left = 100; Top = 100; FontSize = 14 }
        }
    } else {
        return @{ Width = 300; Height = 400; Left = 100; Top = 100; FontSize = 14 }
    }
}
function Save-Config {
    param ($window, $fontSize)
    $config = @{
        Width    = [int]$window.Width
        Height   = [int]$window.Height
        Left     = [int]$window.Left
        Top      = [int]$window.Top
        FontSize = [int]$fontSize
    }
    $config | ConvertTo-Json | Set-Content $configPath
}

# Load note content
function Load-Note {
    if (Test-Path $dataPath) {
        try { (Get-Content $dataPath -Raw | ConvertFrom-Json).text }
        catch { "" }
    } else {
        ""
    }
}
function Save-Note {
    param ($text)
    @{ text = $text } | ConvertTo-Json | Set-Content $dataPath
}

# Create window
$config = Load-Config
$window = New-Object Windows.Window
$window.WindowStyle = 'None'
$window.Title = ""
$window.AllowsTransparency = $false
$window.Opacity = 1
$window.ResizeMode = 'CanResize'
$window.Background = "#80000000"
$window.Width = $config.Width
$window.Height = $config.Height
$window.Left = $config.Left
$window.Top = $config.Top
$window.Topmost = $true
$window.WindowStartupLocation = 'Manual'

# Create Grid layout
$grid = New-Object Windows.Controls.Grid
$grid.Margin = "0"
$grid.RowDefinitions.Clear()

# Create TextBox
$textBox = New-Object Windows.Controls.TextBox
$textBox.Style = $textBoxStyle
$textBox.AcceptsReturn = $true
$textBox.TextWrapping = "NoWrap"
$textBox.HorizontalScrollBarVisibility = "Hidden"
$textBox.VerticalScrollBarVisibility = "Auto"
$textBox.Background = "#80000000"
$textBox.Foreground = "#B3FFFFFF"
$textBox.FontSize = $config.FontSize
$textBox.BorderThickness = 0
$textBox.Margin = "0"
$textBox.Padding = "0"
$textBox.Text = Load-Note
$textBox.VerticalAlignment = "Stretch"
$textBox.HorizontalAlignment = "Stretch"
$textBox.ClipToBounds = $true


# Handle Ctrl + Scroll to zoom text
$textBox.Add_PreviewMouseWheel({
    if ([System.Windows.Input.Keyboard]::IsKeyDown([System.Windows.Input.Key]::LeftCtrl) -or
        [System.Windows.Input.Keyboard]::IsKeyDown([System.Windows.Input.Key]::RightCtrl)) {

        $delta = $_.Delta
        $step = 1.1

        if ($delta -gt 0) {
            $textBox.FontSize = [Math]::Min($textBox.FontSize * $step, 72)
        } elseif ($delta -lt 0) {
            $textBox.FontSize = [Math]::Max($textBox.FontSize / $step, 8)
        }

        Save-Config $window $textBox.FontSize
        $_.Handled = $true
    }
})

# Mouse Double Click Behavior
$textBox.Add_MouseDoubleClick({
    # Capture current vertical scroll offset
    $viewer = $textBox.Template.FindName("PART_ContentHost", $textBox)
    if ($viewer -is [System.Windows.Controls.ScrollViewer]) {
        $offset = $viewer.VerticalOffset
    }

    # Select the line
    $caretIndex = $textBox.CaretIndex
    $text = $textBox.Text

    $start = $text.LastIndexOf("`n", $caretIndex)
    if ($start -eq -1) { $start = 0 } else { $start += 1 }

    $end = $text.IndexOf("`n", $caretIndex)
    if ($end -eq -1) { $end = $text.Length }

    $textBox.Select($start, $end - $start)

    # Restore scroll offset
    if ($viewer -is [System.Windows.Controls.ScrollViewer]) {
        $viewer.ScrollToVerticalOffset($offset)
    }

    $_.Handled = $true
})

# Auto-save every 60 seconds using DispatcherTimer
$autoSaveTimer = New-Object Windows.Threading.DispatcherTimer
$autoSaveTimer.Interval = [TimeSpan]::FromSeconds(30)
$autoSaveTimer.IsEnabled = $true
$autoSaveTimer.Add_Tick({
    Save-Note $textBox.Text
})



# Floating button panel
$buttonPanel = New-Object Windows.Controls.StackPanel
$buttonPanel.Orientation = "Horizontal"
$buttonPanel.HorizontalAlignment = "Right"
$buttonPanel.VerticalAlignment = "Top"
$buttonPanel.Margin = "0,5,5,0"
$buttonPanel.Background = "Transparent"

# Close button
$btnClose = New-Object Windows.Controls.Button
$btnClose.Content = "✕"
$btnClose.Width = 20
$btnClose.Height = 20
$btnClose.Background = "Gray"
$btnClose.Foreground = "Black"
$btnClose.FontWeight = "Bold"
$btnClose.BorderThickness = 0
$btnClose.Cursor = "Hand"
$btnClose.Add_Click({
    Save-Note $textBox.Text
    Save-Config $window $textBox.FontSize
    $window.Close()
})

# Minimize button
$btnMin = New-Object Windows.Controls.Button
$btnMin.Content = "—"
$btnMin.Width = 20
$btnMin.Height = 20
$btnMin.Background = "Gray"
$btnMin.Foreground = "Black"
$btnMin.FontWeight = "Bold"
$btnMin.BorderThickness = 0
$btnMin.Cursor = "Hand"
$btnMin.Add_Click({ $window.WindowState = 'Minimized' })

$buttonPanel.Children.Add($btnMin) | Out-Null
$buttonPanel.Children.Add($btnClose) | Out-Null

# Add controls to grid
$grid.Children.Add($textBox) | Out-Null
$grid.Children.Add($buttonPanel) | Out-Null

# Finalize window
$window.Content = $grid
$window.ShowDialog() | Out-Null