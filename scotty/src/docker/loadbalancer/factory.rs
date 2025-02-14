use scotty_core::settings::loadbalancer::LoadBalancerType;

use super::{haproxy::HaproxyLoadBalancer, traefik::TraefikLoadBalancer, types::LoadBalancerImpl};

pub struct LoadBalancerFactory;

impl LoadBalancerFactory {
    pub fn create(load_balancer_type: &LoadBalancerType) -> Box<dyn LoadBalancerImpl> {
        match load_balancer_type {
            LoadBalancerType::HaproxyConfig => Box::new(HaproxyLoadBalancer),
            LoadBalancerType::Traefik => Box::new(TraefikLoadBalancer),
        }
    }
}
