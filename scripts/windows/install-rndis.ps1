# Binds Windows' own Remote NDIS driver to a G-series pager, for Windows on
# ARM, where Unication's driver package (x86 and x64 only) doesn't install.
#
# Without a driver, Windows on ARM binds the pager as a USB serial port
# (usbser.inf, "USB Serial Device (COMn)"), and Device Manager's
# "Let me pick" and "Have Disk" don't offer Remote NDIS for it. This script
# does through SetupAPI what Unication's INF does on x64: it moves the device
# to the Net class and installs the in-box rndiscmp.inf driver
# ("Remote NDIS Compatible Device"). It downloads and installs nothing else.
#
# Plug in and power on the pager, then from an administrator PowerShell:
#
#   powershell -ExecutionPolicy Bypass -File .\install-rndis.ps1
#
# Run it once per pager. Afterwards `ipconfig` shows a "Remote NDIS
# Compatible Device" adapter on 10.10.0.x. To undo it, uninstall the
# adapter in Device Manager (tick "Attempt to remove the driver") and
# replug the pager.

#Requires -RunAsAdministrator
$ErrorActionPreference = "Stop"

$dev = Get-PnpDevice -PresentOnly | Where-Object { $_.InstanceId -match '^USB\\VID_17FF&PID_A4A2\\' } | Select-Object -First 1
if (-not $dev) { throw "No G-series pager found. Plug it in, turn it on and wait for it to finish booting." }
"Pager: $($dev.FriendlyName) [$($dev.InstanceId)], class $($dev.Class)"
if ($dev.Class -eq "Net") {
  "It's already a network adapter; nothing to do."
  return
}

$inf = Join-Path $env:windir "INF\rndiscmp.inf"
if (-not (Test-Path $inf)) { throw "$inf is missing; this Windows has no in-box Remote NDIS driver." }

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class KnobsetRndis {
  [StructLayout(LayoutKind.Sequential)]
  public struct SP_DEVINFO_DATA { public int cbSize; public Guid ClassGuid; public int DevInst; public IntPtr Reserved; }

  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct SP_DEVINSTALL_PARAMS {
    public int cbSize; public int Flags; public int FlagsEx; public IntPtr hwndParent;
    public IntPtr InstallMsgHandler; public IntPtr InstallMsgHandlerContext; public IntPtr FileQueue;
    public UIntPtr ClassInstallReserved; public int Reserved;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string DriverPath;
  }

  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  public struct SP_DRVINFO_DATA {
    public int cbSize; public int DriverType; public UIntPtr Reserved;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string Description;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string MfgName;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string ProviderName;
    public long DriverDate; public long DriverVersion;
  }

  [DllImport("setupapi.dll", SetLastError = true)]
  public static extern IntPtr SetupDiCreateDeviceInfoList(IntPtr classGuid, IntPtr hwnd);
  [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool SetupDiOpenDeviceInfoW(IntPtr set, string instanceId, IntPtr hwnd, int flags, ref SP_DEVINFO_DATA dev);
  [DllImport("setupapi.dll", SetLastError = true)]
  public static extern bool SetupDiSetDeviceRegistryPropertyW(IntPtr set, ref SP_DEVINFO_DATA dev, int property, byte[] buf, int size);
  [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool SetupDiGetDeviceInstallParamsW(IntPtr set, ref SP_DEVINFO_DATA dev, ref SP_DEVINSTALL_PARAMS p);
  [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool SetupDiSetDeviceInstallParamsW(IntPtr set, ref SP_DEVINFO_DATA dev, ref SP_DEVINSTALL_PARAMS p);
  [DllImport("setupapi.dll", SetLastError = true)]
  public static extern bool SetupDiBuildDriverInfoList(IntPtr set, ref SP_DEVINFO_DATA dev, int driverType);
  [DllImport("setupapi.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool SetupDiEnumDriverInfoW(IntPtr set, ref SP_DEVINFO_DATA dev, int driverType, int index, ref SP_DRVINFO_DATA drv);
  [DllImport("setupapi.dll", SetLastError = true)]
  public static extern bool SetupDiDestroyDeviceInfoList(IntPtr set);
  [DllImport("newdev.dll", SetLastError = true, CharSet = CharSet.Unicode)]
  public static extern bool DiInstallDevice(IntPtr hwnd, IntPtr set, ref SP_DEVINFO_DATA dev, ref SP_DRVINFO_DATA drv, int flags, out bool reboot);
}
"@

$SPDRP_CLASSGUID = 8
$SPDIT_CLASSDRIVER = 1
$DI_ENUMSINGLEINF = 0x10000
$DI_FLAGSEX_ALLOWEXCLUDEDDRVS = 0x800
$NetClass = "{4d36e972-e325-11ce-bfc1-08002be10318}"

function Fail($what) { throw "$what failed: Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())" }
function New-Sized($type) { $o = New-Object $type; $o.cbSize = [Runtime.InteropServices.Marshal]::SizeOf([type]$type); $o }

$set = [KnobsetRndis]::SetupDiCreateDeviceInfoList([IntPtr]::Zero, [IntPtr]::Zero)
try {
  $d = New-Sized "KnobsetRndis+SP_DEVINFO_DATA"
  if (-not [KnobsetRndis]::SetupDiOpenDeviceInfoW($set, $dev.InstanceId, [IntPtr]::Zero, 0, [ref]$d)) { Fail "Opening the device" }

  # Move the device to the Net class, so a network driver can rank for it.
  $guid = [Text.Encoding]::Unicode.GetBytes("$NetClass`0")
  if (-not [KnobsetRndis]::SetupDiSetDeviceRegistryPropertyW($set, [ref]$d, $SPDRP_CLASSGUID, $guid, $guid.Length)) { Fail "Setting the device class" }

  # Look only in rndiscmp.inf, including drivers it excludes from automatic
  # matching (Remote NDIS is one).
  $p = New-Sized "KnobsetRndis+SP_DEVINSTALL_PARAMS"
  if (-not [KnobsetRndis]::SetupDiGetDeviceInstallParamsW($set, [ref]$d, [ref]$p)) { Fail "Reading install parameters" }
  $p.DriverPath = $inf
  $p.Flags = $p.Flags -bor $DI_ENUMSINGLEINF
  $p.FlagsEx = $p.FlagsEx -bor $DI_FLAGSEX_ALLOWEXCLUDEDDRVS
  if (-not [KnobsetRndis]::SetupDiSetDeviceInstallParamsW($set, [ref]$d, [ref]$p)) { Fail "Setting install parameters" }
  if (-not [KnobsetRndis]::SetupDiBuildDriverInfoList($set, [ref]$d, $SPDIT_CLASSDRIVER)) { Fail "Listing drivers" }

  $drv = New-Sized "KnobsetRndis+SP_DRVINFO_DATA"
  $pick = $null
  for ($i = 0; [KnobsetRndis]::SetupDiEnumDriverInfoW($set, [ref]$d, $SPDIT_CLASSDRIVER, $i, [ref]$drv); $i++) {
    if ($drv.Description -match "Remote NDIS") { $pick = $drv; break }
  }
  if (-not $pick) { throw "rndiscmp.inf offers no Remote NDIS driver on this Windows." }

  $reboot = $false
  if (-not [KnobsetRndis]::DiInstallDevice([IntPtr]::Zero, $set, [ref]$d, [ref]$pick, 0, [ref]$reboot)) { Fail "Installing the driver" }
  "Installed: $($pick.Description) ($($pick.MfgName))."
  if ($reboot) { "Windows wants a restart before the adapter works." }
  else { "Give the pager a few seconds, then check with ipconfig for a 10.10.0.x address." }
} finally {
  [void][KnobsetRndis]::SetupDiDestroyDeviceInfoList($set)
}
