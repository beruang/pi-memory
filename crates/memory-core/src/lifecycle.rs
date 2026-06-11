use super::observation::MemoryStatus;

pub fn valid_transition(from: MemoryStatus, to: MemoryStatus) -> bool {
    use MemoryStatus::*;
    match (from, to) {
        (Active, Unconfirmed) => false,
        (Active, Conflicted) => false,
        (Unconfirmed, Active) => true,
        (Unconfirmed, Superseded) => true,
        (Unconfirmed, Obsolete) => true,
        (Unconfirmed, Conflicted) => true,
        (Conflicted, Active) => true,
        (Conflicted, Superseded) => true,
        (Conflicted, Obsolete) => true,
        (Superseded, Obsolete) => true,
        (Superseded, Deleted) => true,
        (Obsolete, Deleted) => true,
        (Deleted, _) => false,
        (_, Deleted) => true,
        (Active, Active) => true,
        (Active, Superseded) => true,
        (Active, Obsolete) => true,
        (from, to) if from == to => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_transitions() {
        assert!(valid_transition(MemoryStatus::Unconfirmed, MemoryStatus::Active));
        assert!(valid_transition(MemoryStatus::Active, MemoryStatus::Superseded));
        assert!(valid_transition(MemoryStatus::Active, MemoryStatus::Obsolete));
        assert!(valid_transition(MemoryStatus::Superseded, MemoryStatus::Deleted));
        assert!(valid_transition(MemoryStatus::Conflicted, MemoryStatus::Active));
        assert!(valid_transition(MemoryStatus::Active, MemoryStatus::Deleted));
    }

    #[test]
    fn test_same_status_allowed() {
        assert!(valid_transition(MemoryStatus::Active, MemoryStatus::Active));
        assert!(valid_transition(MemoryStatus::Unconfirmed, MemoryStatus::Unconfirmed));
    }

    #[test]
    fn test_disallowed_transitions() {
        assert!(!valid_transition(MemoryStatus::Deleted, MemoryStatus::Active));
        assert!(!valid_transition(MemoryStatus::Deleted, MemoryStatus::Obsolete));
    }

    #[test]
    fn test_soft_delete_is_terminal() {
        for from in [MemoryStatus::Deleted].iter() {
            for to in [MemoryStatus::Active, MemoryStatus::Unconfirmed, MemoryStatus::Superseded, MemoryStatus::Obsolete, MemoryStatus::Conflicted].iter() {
                assert!(!valid_transition(*from, *to), "deleted -> {:?} should be invalid", to);
            }
        }
    }
}
