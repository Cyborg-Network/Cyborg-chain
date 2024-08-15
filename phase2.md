### Milestone 1 - Cyborg Oracle Implementation

- **Estimated duration:** 1 month

#### **Deliverables:**

| Number | Completed |Deliverable | Specification |
| -----: | ----- | ----------- | ------------- |
| **0a.** | -[] | License | GPLv3 |
| **0b.** | Documentation | We will provide both **inline documentation** of the code and a basic **tutorial** that explains how users can (for example) deploy docker images using our interface. |
| **0c.** | Testing and Testing Guide | Core functions will be fully covered by comprehensive unit tests to ensure functionality and robustness. In the guide, we will describe how to run these tests. |
| **0d.** | Docker | We will provide a Dockerfile(s) that can be used to deploy a local cyborg parachain and test the features of this delivery. |
| 1. | Working Demo | We will provide video documentation to help developers understand the process of testing the Orcale implementation.|
| 2. | Substrate Module: Oracle | This pallet will be responsible for establishing communication and regulating the use of an oracle. |
| 3. | Front end App: Cyborg Connect | Updated UI with features to display Task verification and result information from Orcale |
| 4. | Cyborg Oracle | An ORML based oracle implementation to seamless exchange data packets between cyborg parachain and connected offchain clusters|


### Milestone 2 - Cyborg Worker Node Implementation

- **Estimated duration:** 1 month

#### **Deliverables:**

| Number | Deliverable | Specification |
| -----: | ----------- | ------------- |
| **0a.** | License | GPLv3 |
| **0b.** | Documentation | We will provide both **inline documentation** of the code and a basic **tutorial** that explains how users can launch a cyborg worker node using their local local machine|
| **0c.** | Testing and Testing Guide | Core functions will be fully covered by comprehensive unit tests to ensure functionality and robustness. In the guide, we will describe how to run these tests. |
| **0d.** | Docker | We will provide a Dockerfile(s) that can be used to run a cyborg worker node locally. |
| 1. | Working Demo | We will provide video documentation to help developers understand the process of connecting and managing a cyborg worker node.|
| 2. | Substrate Module: Payments | This pallet will be responsible for estimating the costs for executing a task based on server specifications and time. |
| 3. | Substrate Module: Inventory | This pallet will be responsible for uniquely mapping each worker node uniquely in the onchain inventory to keep track of server status. |
| 4. | Substrate Node implementation: Cyborg Worker | A custom node implementation to contribute compute power to the cyborg parachain and earn rewards for contributions|
| 5. | Cyborg Connect: Provide Compute (Feature)  | The UI implementation to support deployment and management of Cyborg worker nodes|

### Milestone 3 - ZK Verification Layer

- **Estimated duration:** 1 month

#### **Deliverables:**

| Number | Deliverable | Specification |
| -----: | ----------- | ------------- |
| **0a.** | License | GPLv3 |
| **0b.** | Documentation | We will provide both **inline documentation** of the code and a basic **tutorial** that explains how users can (for example) deploy apps using yaml files through our interface |
| **0c.** | Testing and Testing Guide | Core functions will be fully covered by comprehensive unit tests to ensure functionality and robustness. In the guide, we will describe how to run these tests. |
| **0d.** | Docker | We will provide a Dockerfile(s) that can be used to test all the functionality delivered with this milestone. |
| **0e.** | Article | We will publish a medium blog explaining the vision of NueroZK, current features and future plans |
| 1. | Testing suite | We will provide a testing facility to examine the working of ZK verification tool with the Cyborg parachain.|
| 2. | Substrate Module: ZK verifier | This pallet will be responsible for verifying and confirming proofs emitted by the offchain ZK worker through the cyborg oracle. |
| 3. | Cyborg ZK worker | A feature to the Cyborg worker node to posses specific instructions about ZK proof generation for a specific executed AI algorithm|
| 4. | Substrate module: Worker Rewards | A custom pallet that holds the logic of assigning rewards to commited worker nodes for executing user tasks |