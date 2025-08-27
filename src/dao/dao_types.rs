//! DAO types and data structures

use serde::{Deserialize, Serialize};
use lib_crypto::Hash;
use lib_identity::IdentityId;

/// DAO proposal for governance decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoProposal {
    /// Unique proposal identifier
    pub id: Hash,
    /// Proposal title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Proposer identity
    pub proposer: IdentityId,
    /// Type of proposal
    pub proposal_type: DaoProposalType,
    /// Current status
    pub status: DaoProposalStatus,
    /// Voting start time
    pub voting_start_time: u64,
    /// Voting end time
    pub voting_end_time: u64,
    /// Minimum quorum required (percentage)
    pub quorum_required: u8,
    /// Current vote tally
    pub vote_tally: DaoVoteTally,
    /// Proposal creation timestamp
    pub created_at: u64,
    /// Block height when proposal was created
    pub created_at_height: u64,
    /// Execution parameters (if passed)
    pub execution_params: Option<Vec<u8>>,
}

/// Types of DAO proposals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DaoProposalType {
    /// Universal Basic Income parameter changes
    UbiDistribution,
    /// Protocol upgrade proposals
    ProtocolUpgrade,
    /// Treasury fund allocation
    TreasuryAllocation,
    /// Validator set changes
    ValidatorUpdate,
    /// Economic parameter adjustments
    EconomicParams,
    /// Network governance rules
    GovernanceRules,
    /// Modify transaction fee structure
    FeeStructure,
    /// Emergency protocol changes
    Emergency,
    /// Community development funds
    CommunityFunding,
    /// Research and development grants
    ResearchGrants,
}

/// DAO proposal status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DaoProposalStatus {
    /// Proposal is in draft state
    Draft,
    /// Proposal is active and accepting votes
    Active,
    /// Proposal has passed and is ready for execution
    Passed,
    /// Proposal has failed (rejected or insufficient quorum)
    Failed,
    /// Proposal has been executed
    Executed,
    /// Proposal has been cancelled
    Cancelled,
    /// Proposal has expired without sufficient participation
    Expired,
}

/// Vote tally for a DAO proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoVoteTally {
    /// Total number of votes cast
    pub total_votes: u64,
    /// Number of "yes" votes
    pub yes_votes: u64,
    /// Number of "no" votes
    pub no_votes: u64,
    /// Number of "abstain" votes
    pub abstain_votes: u64,
    /// Total eligible voting power
    pub total_eligible_power: u64,
    /// Weighted yes votes (considering voting power)
    pub weighted_yes: u64,
    /// Weighted no votes (considering voting power)
    pub weighted_no: u64,
    /// Weighted abstain votes (considering voting power)
    pub weighted_abstain: u64,
}

/// Individual DAO vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoVote {
    /// Vote identifier
    pub id: Hash,
    /// Proposal being voted on
    pub proposal_id: Hash,
    /// Voter identity
    pub voter: Hash,
    /// Vote choice
    pub vote_choice: DaoVoteChoice,
    /// Voting power used
    pub voting_power: u64,
    /// Vote timestamp
    pub timestamp: u64,
    /// Vote signature
    pub signature: lib_crypto::Signature,
    /// Optional justification for the vote
    pub justification: Option<String>,
}

/// DAO vote choices
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DaoVoteChoice {
    /// Vote in favor of the proposal
    Yes,
    /// Vote against the proposal
    No,
    /// Abstain from voting (counted for quorum but not for/against)
    Abstain,
    /// Delegate vote to another participant
    Delegate(IdentityId),
}

impl DaoVoteChoice {
    /// Convert vote choice to u8 for serialization
    pub fn to_u8(&self) -> u8 {
        match self {
            DaoVoteChoice::Yes => 1,
            DaoVoteChoice::No => 2,
            DaoVoteChoice::Abstain => 3,
            DaoVoteChoice::Delegate(_) => 4,
        }
    }
}

/// DAO treasury management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaoTreasury {
    /// Total treasury balance (ZHTP tokens)
    pub total_balance: u64,
    /// Available balance for allocation
    pub available_balance: u64,
    /// Currently allocated funds
    pub allocated_funds: u64,
    /// Reserved funds (cannot be allocated)
    pub reserved_funds: u64,
    /// Treasury transaction history
    pub transaction_history: Vec<TreasuryTransaction>,
    /// Annual budget allocations
    pub annual_budgets: Vec<AnnualBudget>,
}

/// Treasury transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransaction {
    /// Transaction identifier
    pub id: Hash,
    /// Transaction type
    pub transaction_type: TreasuryTransactionType,
    /// Amount transferred
    pub amount: u64,
    /// Recipient (for outgoing transactions)
    pub recipient: Option<IdentityId>,
    /// Source (for incoming transactions)
    pub source: Option<IdentityId>,
    /// Associated proposal (if any)
    pub proposal_id: Option<Hash>,
    /// Transaction timestamp
    pub timestamp: u64,
    /// Transaction description
    pub description: String,
}

/// Types of treasury transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TreasuryTransactionType {
    /// Incoming funds (from protocol fees, donations, etc.)
    Deposit,
    /// Outgoing allocation to approved proposal
    Allocation,
    /// UBI distribution
    UbiDistribution,
    /// Validator rewards
    ValidatorRewards,
    /// Emergency fund usage
    Emergency,
    /// Community development funding
    CommunityFunding,
    /// Research grants
    ResearchGrant,
}

/// Annual budget allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnualBudget {
    /// Budget year
    pub year: u32,
    /// Total allocated budget
    pub total_allocation: u64,
    /// UBI allocation
    pub ubi_allocation: u64,
    /// Community development allocation
    pub community_allocation: u64,
    /// Research and development allocation
    pub research_allocation: u64,
    /// Emergency reserve allocation
    pub emergency_allocation: u64,
    /// Validator incentive allocation
    pub validator_allocation: u64,
    /// Spent amount so far
    pub spent_amount: u64,
    /// Budget approval proposal ID
    pub approval_proposal_id: Hash,
}

impl Default for DaoVoteTally {
    fn default() -> Self {
        Self {
            total_votes: 0,
            yes_votes: 0,
            no_votes: 0,
            abstain_votes: 0,
            total_eligible_power: 0,
            weighted_yes: 0,
            weighted_no: 0,
            weighted_abstain: 0,
        }
    }
}

impl DaoVoteTally {
    /// Calculate approval percentage
    pub fn approval_percentage(&self) -> f64 {
        if self.total_votes == 0 {
            return 0.0;
        }
        (self.yes_votes as f64 / self.total_votes as f64) * 100.0
    }
    
    /// Calculate quorum percentage
    pub fn quorum_percentage(&self) -> f64 {
        if self.total_eligible_power == 0 {
            return 0.0;
        }
        (self.total_votes as f64 / self.total_eligible_power as f64) * 100.0
    }
    
    /// Calculate weighted approval percentage
    pub fn weighted_approval_percentage(&self) -> f64 {
        let total_weighted = self.weighted_yes + self.weighted_no;
        if total_weighted == 0 {
            return 0.0;
        }
        (self.weighted_yes as f64 / total_weighted as f64) * 100.0
    }
}
