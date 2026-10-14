/// Force-branch and wheel-speed integration policy. See the tire-profile specification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TireIntegration {
    /// Blend force branches and regularize low-speed slip and brake torque for generic fixtures.
    #[default]
    Regularized,
    /// Use the reference gripping/sliding branch and wheel-acceleration integration.
    Reference,
}
