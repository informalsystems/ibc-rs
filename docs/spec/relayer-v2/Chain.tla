---------------------------- MODULE Chain ----------------------------

EXTENDS Naturals, FiniteSets, ClientHandlers, ConnectionHandlers, ChannelHandlers

CONSTANTS ChainID, 
          MaxHeight \* maximal height of all the chains in the system 

VARIABLES chainStore,
          incomingDatagrams

vars == <<chainStore, incomingDatagrams>>          
Heights == 1..MaxHeight
nullHeight == 0


(***************************************************************************
 Client update operators
 ***************************************************************************)
\* Update the clients on chain with chainID, 
\* using the client datagrams generated by the relayer      
\* (Handler operators defined in ClientHandlers.tla)
LightClientUpdate(chainID, chain, datagrams) == 
    \* create clients
    LET clientCreatedChain == HandleCreateClient(chainID, chain, datagrams) IN
    \* update clients
    LET clientUpdatedChain == HandleUpdateClient(chainID, clientCreatedChain, datagrams) IN

    clientUpdatedChain
    
(***************************************************************************
 Connection update operators
 ***************************************************************************)
\* Update the connections on chain with chainID, 
\* using the connection datagrams generated by the relayer
\* (Handler operators defined in ConnectionHandlers.tla)
ConnectionUpdate(chainID, chain, datagrams) ==
    \* update chain with "ConnOpenInit" datagrams
    LET connOpenInitChain == HandleConnOpenInit(chainID, chain, datagrams) IN
    \* update chain with "ConnOpenTry" datagrams
    LET connOpenTryChain == HandleConnOpenTry(chainID, connOpenInitChain, datagrams) IN
    \* update chain with "ConnOpenAck" datagrams
    LET connOpenAckChain == HandleConnOpenAck(chainID, connOpenTryChain, datagrams) IN
    \* update chain with "ConnOpenConfirm" datagrams
    LET connOpenConfirmChain == HandleConnOpenConfirm(chainID, connOpenAckChain, datagrams) IN
    
    connOpenConfirmChain

(***************************************************************************
 Channel update operators
 ***************************************************************************)
\* Update the channels on chain with chainID, 
\* using the channel datagrams generated by the relayer
\* (Handler operators defined in ChannelHandlers.tla)
ChannelUpdate(chainID, chain, datagrams) ==
    \* update chain with "ChanOpenInit" datagrams
    LET chanOpenInitChain == HandleChanOpenInit(chainID, chain, datagrams) IN
    \* update chain with "ChanOpenTry" datagrams
    LET chanOpenTryChain == HandleChanOpenTry(chainID, chanOpenInitChain, datagrams) IN
    \* update chain with "ChanOpenAck" datagrams
    LET chanOpenAckChain == HandleChanOpenAck(chainID, chanOpenTryChain, datagrams) IN
    \* update chain with "ChanOpenConfirm" datagrams
    LET chanOpenConfirmChain == HandleChanOpenConfirm(chainID, chanOpenAckChain, datagrams) IN
    
    chanOpenConfirmChain

(***************************************************************************
 Chain update operators
 ***************************************************************************)
\* Update chainID with the received datagrams
\* Supports ICS2 (Clients), ICS3 (Connections), and ICS4 (Channels).
UpdateChain(chainID, datagrams) == 
    \* ICS 002: Client updates
    LET lightClientsUpdated == LightClientUpdate(chainID, chain, datagrams) IN 
    \* ICS 003: Connection updates
    LET connectionsUpdated == ConnectionUpdate(chainID, lightClientsUpdated, datagrams) IN
    \* ICS 004: Channel updates
    LET channelsUpdated == ChannelUpdate(chainID, connectionsUpdated, datagrams) IN
    
    \* update height
    LET updatedChain == 
        IF /\ chain /= channelsUpdated
           /\ chain.height < MaxHeight 
        THEN [channelsUpdated EXCEPT !.height = chain.height + 1]
        ELSE channelsUpdated
    IN
    
    updatedChain

(***************************************************************************
 Chain actions
 ***************************************************************************)       
\* Advance the height of the chain until MaxHeight is reached
AdvanceChain ==
    /\ chain.height < MaxHeight
    /\ chain' = [chain EXCEPT !height = chain.height + 1]
    /\ UNCHANGED incomingDatagrams

\* Receive the datagrams and update the chain state        
ReceiveIncomingDatagrams ==
    /\ incomingDatagrams /= {} 
    /\ chain' = UpdateChain(ChainID, incomingDatagrams[chainID])
    /\ incomingDatagrams' = {}

(***************************************************************************
 Specification
 ***************************************************************************)
\* Initial state predicate
\* Initially
\*  - each chain is initialized to InitChain (defined in RelayerDefinitions.tla)
\*  - pendingDatagrams for each chain is empty
Init == 
    /\ chainStore = InitChain 
    /\ incomingDatagrams = {}

\* Next state action
\* The chain either
\*  - advances its height
\*  - receives datagrams and updates its state
Next ==
    \/ AdvanceChain
    \/ HandleIncomingDatagrams
    \/ UNCHANGED vars
        
\* Fairness constraints 
Fairness ==
    /\ WF_vars(AdvanceChain)
    /\ WF_vars(HandleIncomingDatagrams)

(***************************************************************************
 Invariants
 ***************************************************************************)
\* Type invariant   
\* Chains and Datagrams are defined in RelayerDefinitions.tla        
TypeOK ==    
    /\ chainStore \in Chains
    /\ incomingDatagrams \in SUBSET Datagrams

=============================================================================
\* Modification History
\* Last modified Mon Jun 08 16:48:22 CET 2020 by ilinastoilkovska
\* Created Fri Jun 05 16:56:21 CET 2020 by ilinastoilkovska
