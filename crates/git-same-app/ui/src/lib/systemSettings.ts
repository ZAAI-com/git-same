// System Settings deep links. The `x-apple.systempreferences:` scheme is the
// documented way to open the Login Items & Extensions pane and the Privacy &
// Security > Full Disk Access pane; current macOS still honours these anchors.
export const EXTENSIONS_URL =
  'x-apple.systempreferences:com.apple.LoginItems-Settings.extension';
export const FDA_URL =
  'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles';
