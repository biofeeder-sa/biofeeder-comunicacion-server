
/// Enum for message mode, only this modes is required
pub enum MessageMode{
    ModeReadResponse,
    ModeWriteResponse,
    ModeLogResponse,
    LogRecovery,
    XbeeError,
    HydrophoneError
}
