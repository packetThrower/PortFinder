package main

import (
	"context"
	"sync"

	"PortFinder/backend/capture"
	"PortFinder/backend/privilege"
)

type App struct {
	ctx        context.Context
	cancelFunc context.CancelFunc
	mu         sync.Mutex
}

func NewApp() *App {
	return &App{}
}

func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
	a.cancelFunc = nil
}

func (a *App) shutdown(_ context.Context) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.cancelFunc != nil {
		a.cancelFunc()
	}
}

func (a *App) GetInterfaces() ([]capture.InterfaceInfo, error) {
	return capture.ListInterfaces()
}

func (a *App) StartCapture(request capture.CaptureRequest) (*capture.CaptureResult, error) {
	a.mu.Lock()
	if a.cancelFunc != nil {
		a.cancelFunc()
	}
	ctx, cancel := context.WithCancel(a.ctx)
	a.cancelFunc = cancel
	a.mu.Unlock()

	result, err := capture.Run(ctx, request)

	a.mu.Lock()
	a.cancelFunc = nil
	a.mu.Unlock()

	return result, err
}

func (a *App) StopCapture() {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.cancelFunc != nil {
		a.cancelFunc()
		a.cancelFunc = nil
	}
}

func (a *App) GetVersion() string {
	return Version
}

func (a *App) CheckPrivileges() bool {
	return privilege.HasCapturePrivilege()
}
