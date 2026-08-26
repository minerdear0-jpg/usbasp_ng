Subject: Re: classic MS OS 2.0 nesting — Win11 A/B (device-level kept)

Hi,

Thank you again for the RC1 review. The string-index fix (USB_STR_* vs
V-USB PROP flags) was exactly right and stays.

On classic MS OS nesting we now have a clean hardware A/B, so I want to
phrase the conclusion carefully — not as “nested is wrong / device-level
is right in general,” but as a Windows compatibility decision for this
product:

  For USBasp NG classic, as a non-composite single-function device, the
  device-level MS OS 2.0 layout was compatible with Windows 11 automatic
  WinUSB binding, whereas the configuration/function subset layout on the
  same device left the stick unbound.

Evidence (same yellow-dot stick, same VID/PID, same protocol, BOS present;
Linux vendor GET returned a full WINUSB blob in both cases):

  MS OS 2.0 = 0x9E (device-level)     → Win11 WinUSB ✓
  MS OS 2.0 = 0xAE (config/function) → Win11 yellow bang, no publisher ✗

We are keeping classic on:

  Set → Compatible ID WINUSB → DeviceInterfaceGUID (REG_SZ)
  total 0x9E

and treating that as part of the classic compatibility contract
(docs/USB_WINDOWS.md), with CI asserting: classic has no Configuration /
Function subsets; HIDUART keeps nested subsets (0xB2) for its composite
topology.

bcdDevice 2.03 is the release identity for this verified classic line
(cache-bust after the failed nested bind). We will not bump bcdDevice on
every descriptor tweak.

Your earlier nesting recommendation for classic is withdrawn in light of
this bench. Nested remains appropriate for HIDUART.

Best regards
