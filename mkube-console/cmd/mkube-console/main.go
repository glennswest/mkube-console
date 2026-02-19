package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/glennswest/mkube-console/config"
	"github.com/glennswest/mkube-console/internal/api"
	"github.com/glennswest/mkube-console/internal/cluster"
)

func main() {
	configPath := flag.String("config", "/etc/mkube-console/config.yaml", "Path to configuration file")
	flag.Parse()

	cfg, err := config.Load(*configPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error loading config: %v\n", err)
		os.Exit(1)
	}

	ctx, cancel := signal.NotifyContext(context.Background(), syscall.SIGINT, syscall.SIGTERM)
	defer cancel()

	// Build node clients
	var clients []*cluster.NodeClient
	for _, n := range cfg.Nodes {
		clients = append(clients, cluster.NewNodeClient(n.Name, n.Address))
	}

	if len(clients) == 0 {
		log.Fatal("no nodes configured")
	}

	// Create aggregator
	agg := cluster.NewAggregator(clients)
	go agg.RunHealthChecker(ctx)

	// Create API router
	router := api.NewRouter(agg, cfg.RegistryURL, cfg.LogsURL)

	// Register routes on a shared mux
	mux := http.NewServeMux()
	router.RegisterRoutes(mux)

	srv := &http.Server{
		Addr:    cfg.ListenAddr,
		Handler: mux,
	}

	go func() {
		<-ctx.Done()
		shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer shutdownCancel()
		_ = srv.Shutdown(shutdownCtx)
	}()

	log.Printf("mkube-console listening on %s", cfg.ListenAddr)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("server error: %v", err)
	}
}
