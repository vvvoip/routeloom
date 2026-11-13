import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private var tunnelBridge: TunnelBridge?
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    tunnelBridge = TunnelBridge(messenger: flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
  }
}
