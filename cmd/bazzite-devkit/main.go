package main

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"

	"github.com/lobinuxsoft/bazzite-devkit/internal/ui"
)

func main() {
	a := app.New()
	w := a.NewWindow("Bazzite Devkit")
	w.Resize(fyne.NewSize(800, 600))

	ui.Setup(w)

	w.ShowAndRun()
}
