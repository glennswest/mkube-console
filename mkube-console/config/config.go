package config

import (
	"fmt"
	"os"

	"gopkg.in/yaml.v3"
)

// Config is the top-level configuration for mkube-console.
type Config struct {
	ListenAddr   string    `yaml:"listenAddr"`   // ":9090"
	Nodes        []NodeDef `yaml:"nodes"`        // Static node list
	DiscoveryDNS string    `yaml:"discoveryDNS"` // "_mkube._tcp.gt.lo"
	RegistryURL  string    `yaml:"registryURL"`  // fastregistry base URL
	LogsURL      string    `yaml:"logsURL"`      // micrologs base URL
}

// NodeDef defines a single mkube node to connect to.
type NodeDef struct {
	Name    string `yaml:"name"`
	Address string `yaml:"address"` // "http://192.168.200.2:8082"
}

// Load reads the config from a YAML file.
func Load(path string) (*Config, error) {
	cfg := &Config{
		ListenAddr: ":9090",
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading config %s: %w", path, err)
	}

	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("parsing config: %w", err)
	}

	if len(cfg.Nodes) == 0 && cfg.DiscoveryDNS == "" {
		return nil, fmt.Errorf("at least one node or discoveryDNS must be configured")
	}

	return cfg, nil
}
