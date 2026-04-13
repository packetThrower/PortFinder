package main

import (
	"embed"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	"github.com/wailsapp/wails/v2/pkg/options/mac"
)

//go:embed all:frontend/dist
var assets embed.FS

var Version = "dev"

func main() {
	app := NewApp()

	err := wails.Run(&options.App{
		Title:     "PortFinder",
		Width:     480,
		Height:    560,
		MinWidth:  400,
		MinHeight: 480,
		AssetServer: &assetserver.Options{
			Assets: assets,
		},
		BackgroundColour: &options.RGBA{R: 80, G: 80, B: 80, A: 1},
		OnStartup:        app.startup,
		OnShutdown:       app.shutdown,
		Bind: []interface{}{
			app,
		},
		Mac: &mac.Options{
			TitleBar: &mac.TitleBar{
				TitlebarAppearsTransparent: true,
			},
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
