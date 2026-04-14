package main

import (
	"embed"
	"os"
	"runtime"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/options/mac"
)

//go:embed all:frontend/dist
var assets embed.FS

var Version = "dev"

func main() {
	// Suppress WebKitGTK signal handler warning on Linux
	if runtime.GOOS == "linux" {
		os.Setenv("JSC_SIGNAL_FOR_GC", "20")
	}
	app := NewApp()

	err := wails.Run(&options.App{
		Title:     "PortFinder",
		Width:     480,
		Height:    460,
		MinWidth:  400,
		MinHeight: 460,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 0, G: 24, B: 37, A: 1},
		OnStartup:        app.startup,
		OnShutdown:       app.shutdown,
		Bind: []interface{}{
			app,
		},
		Mac: &mac.Options{
			TitleBar: mac.TitleBarDefault(),
			About: &mac.AboutInfo{
				Title:   "PortFinder",
				Message: "Network switch port discovery tool\nVersion " + Version,
			},
		},
	})

	if err != nil {
		println("Error:", err.Error())
	}
}
